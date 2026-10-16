use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use indexmap::IndexMap;
use petgraph::graph::{DiGraph, NodeIndex};

use stratum_core::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId},
    types::{Container, ContainerKind, Layer, Module},
    violation::SourceLocation,
};

use crate::index::{DirEntries, DirRole, GraphIndex, SliceImporters, slice_importers};

/// The compound graph view used by rules, the LSP, and the visualizer.
///
/// Three views over the same node set:
/// 1. Dependency DAG — `petgraph::DiGraph<ModuleId, EdgeKind>` (may contain
///    cycles; [`crate::cycles::find_cycles`] reports them).
/// 2. Nesting tree — modules belong to containers; layers are top-level containers.
/// 3. Visibility scopes — each `Module` carries its own `VisibilityScope`.
#[derive(Debug, Clone)]
pub struct CompoundGraph {
    pub modules: IndexMap<ModuleId, Module>,
    pub containers: IndexMap<ContainerId, Container>,
    pub dir_roles: BTreeMap<PathBuf, DirRole>,
    pub layers: IndexMap<LayerId, Layer>,
    pub deps: DiGraph<ModuleId, EdgeKind>,
    pub node_index: rustc_hash::FxHashMap<ModuleId, NodeIndex>,
    pub entry_points: Vec<ModuleId>,
    /// Navigation docs by folder ; empty unless `stratum/module-docs` sets `nav_file`.
    pub nav_docs: BTreeMap<PathBuf, NavDoc>,
    /// Built on first read from module paths, visibility and containers. Incremental updates
    /// change none of these; a change to them needs a new graph, or the index goes stale.
    index: OnceLock<GraphIndex>,
    slice_importers: OnceLock<SliceImporters>,
}

impl CompoundGraph {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            dir_roles: BTreeMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: rustc_hash::FxHashMap::default(),
            entry_points: Vec::new(),
            nav_docs: BTreeMap::new(),
            index: OnceLock::new(),
            slice_importers: OnceLock::new(),
        }
    }

    pub(crate) fn index(&self) -> &GraphIndex {
        self.index.get_or_init(|| GraphIndex::of(self))
    }

    pub(crate) fn replace_deps(&mut self, deps: DiGraph<ModuleId, EdgeKind>) {
        self.deps = deps;
        self.slice_importers = OnceLock::new();
    }

    /// Where `from` first imports `to`.
    #[must_use]
    pub fn import_location(&self, from: ModuleId, to: ModuleId) -> Option<SourceLocation> {
        self.modules
            .get(&from)?
            .imports
            .iter()
            .find(|i| i.target == Some(to))
            .map(|i| i.location)
    }

    /// The other slices holding a module that imports a module of `slice`, by any edge kind.
    #[must_use]
    pub fn importing_slices(&self, slice: ContainerId) -> &BTreeSet<ContainerId> {
        static NONE: BTreeSet<ContainerId> = BTreeSet::new();
        self.slice_importers
            .get_or_init(|| slice_importers(self))
            .get(&slice)
            .unwrap_or(&NONE)
    }

    /// Modules held by `c` or by any container nested in it.
    #[must_use]
    pub fn subtree_module_count(&self, c: ContainerId) -> usize {
        self.index().subtree_modules.get(&c).copied().unwrap_or(0)
    }

    #[must_use]
    pub fn dir_entries(&self, dir: &Path) -> Option<&DirEntries> {
        self.index().dirs.get(dir)
    }

    #[must_use]
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.deps.edge_count()
    }

    /// Resolve a `ModuleId` to its petgraph `NodeIndex`.
    #[must_use]
    pub fn node_for(&self, m: ModuleId) -> Option<NodeIndex> {
        self.node_index.get(&m).copied()
    }

    /// From the innermost container of `m` out to its `Layer` container.
    pub fn containers_of(&self, m: ModuleId) -> impl Iterator<Item = &Container> {
        let innermost = self
            .modules
            .get(&m)
            .and_then(|m| self.containers.get(&m.container));
        std::iter::successors(innermost, |c| {
            c.parent.and_then(|p| self.containers.get(&p))
        })
    }

    /// The slice holding `m`, else its `Layer` container.
    #[must_use]
    pub fn outer_container(&self, m: ModuleId) -> Option<&Container> {
        let mut outer = None;
        for c in self.containers_of(m) {
            if c.kind == ContainerKind::Slice {
                return Some(c);
            }
            outer = Some(c);
        }
        outer
    }

    /// The innermost container enclosing `path`, which need not exist on disk.
    #[must_use]
    pub fn container_at(&self, path: &Path) -> Option<&Container> {
        self.containers
            .values()
            .filter(|c| path.starts_with(&c.path))
            .max_by_key(|c| c.path.components().count())
    }

    /// Every private-zone folder that holds modules, in path order. `files` excludes
    /// modules of a zone nested inside it.
    #[must_use]
    pub fn private_zones(&self) -> &[PrivateZone] {
        &self.index().zones
    }

    /// The private zones whose innermost enclosing container is `c`, in path order.
    pub fn private_zones_in(&self, c: ContainerId) -> impl Iterator<Item = &PrivateZone> {
        let index = self.index();
        index
            .zones
            .iter()
            .zip(&index.zone_containers)
            .filter(move |(_, at)| **at == Some(c))
            .map(|(z, _)| z)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateZone {
    pub path: PathBuf,
    pub root: PathBuf,
    pub files: Vec<ModuleId>,
}

/// A folder's navigation doc, read by the engine ; `requires` holds folders, not doc files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NavDoc {
    pub requires: Vec<PathBuf>,
}

impl Default for CompoundGraph {
    fn default() -> Self {
        Self::empty()
    }
}
