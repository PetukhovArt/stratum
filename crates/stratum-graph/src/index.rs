use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;
use stratum_core::{ids::ContainerId, types::ContainerKind, visibility::VisibilityScope};

use crate::graph::{CompoundGraph, PrivateZone};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GraphIndex {
    pub(crate) subtree_modules: FxHashMap<ContainerId, usize>,
    pub(crate) dirs: FxHashMap<PathBuf, DirEntries>,
    pub(crate) zones: Vec<PrivateZone>,
    pub(crate) zone_containers: Vec<Option<ContainerId>>,
    entry_stems: FxHashMap<PathBuf, String>,
}

/// What a folder inside a layer is. Placement decides it from `structure` and the file set; no
/// rule derives a role from a folder name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirRole {
    LayerRoot,
    Container(ContainerId),
    Segment,
    PrivateZone,
    /// A folder without an entry file that is none of the others: its files belong to the
    /// enclosing container.
    Collection,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirEntries {
    pub files: Vec<String>,
    pub dirs: BTreeSet<String>,
}

impl GraphIndex {
    pub(crate) fn of(g: &CompoundGraph) -> Self {
        let mut subtree_modules: FxHashMap<ContainerId, usize> = FxHashMap::default();
        let mut dirs: FxHashMap<PathBuf, DirEntries> = FxHashMap::default();
        for m in g.modules.values() {
            let enclosing = g.containers_of(m.id).skip(1).map(|c| c.id);
            for c in std::iter::once(m.container).chain(enclosing) {
                *subtree_modules.entry(c).or_default() += 1;
            }
            let mut inner = m.path.as_path();
            while let (Some(outer), Some(name)) = (inner.parent(), inner.file_name()) {
                if outer.as_os_str().is_empty() {
                    break;
                }
                let name = name.to_string_lossy().into_owned();
                let entries = dirs.entry(outer.to_path_buf()).or_default();
                if inner == m.path {
                    entries.files.push(name);
                } else if !entries.dirs.insert(name) {
                    break;
                }
                inner = outer;
            }
        }
        let zones = private_zones(g);
        let zone_containers = zones
            .iter()
            .map(|z| g.container_at(&z.path).map(|c| c.id))
            .collect();
        let entry_stems = g
            .containers
            .values()
            .filter_map(|c| {
                let name = g.modules.get(&c.entry?)?.path.file_name()?;
                Some((c.path.clone(), stem(&name.to_string_lossy())?))
            })
            .collect();
        Self {
            subtree_modules,
            dirs,
            zones,
            zone_containers,
            entry_stems,
        }
    }
}

impl CompoundGraph {
    /// The distinct things directly inside `dir`: each child folder, and the child files sharing a
    /// stem before the first dot (`foo.ts`, `foo.test.ts` and `foo.vue` are one kind). The folder's
    /// entry file and private zones do not count.
    #[must_use]
    pub fn kinds(&self, dir: &Path) -> BTreeSet<String> {
        let mut names = self.child_names(dir);
        if let Some(entry) = self.index().entry_stems.get(dir) {
            names.remove(entry);
        }
        names
    }

    /// The size of a homogeneous folder: the things directly inside `dir` counted as in
    /// [`Self::kinds`], entry file included.
    #[must_use]
    pub fn items(&self, dir: &Path) -> usize {
        self.child_names(dir).len()
    }

    fn child_names(&self, dir: &Path) -> BTreeSet<String> {
        let Some(entries) = self.dir_entries(dir) else {
            return BTreeSet::new();
        };
        let folders = entries
            .dirs
            .iter()
            .filter(|name| self.dir_roles.get(&dir.join(name)) != Some(&DirRole::PrivateZone))
            .map(|name| format!("{name}/"));
        let files = entries.files.iter().filter_map(|name| stem(name));
        folders.chain(files).collect()
    }
}

fn stem(file_name: &str) -> Option<String> {
    file_name
        .split('.')
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

pub(crate) type SliceImporters = FxHashMap<ContainerId, BTreeSet<ContainerId>>;

pub(crate) fn slice_importers(g: &CompoundGraph) -> SliceImporters {
    let slice_of: FxHashMap<_, _> = g
        .modules
        .keys()
        .filter_map(|&m| {
            let c = g.outer_container(m)?;
            (c.kind == ContainerKind::Slice).then_some((m, c.id))
        })
        .collect();
    let mut importers = SliceImporters::default();
    for e in g.deps.raw_edges() {
        let from = slice_of.get(&g.deps[e.source()]);
        let to = slice_of.get(&g.deps[e.target()]);
        if let (Some(&from), Some(&to)) = (from, to) {
            if from != to {
                importers.entry(to).or_default().insert(from);
            }
        }
    }
    importers
}

fn private_zones(g: &CompoundGraph) -> Vec<PrivateZone> {
    let mut zones: BTreeMap<PathBuf, PrivateZone> = BTreeMap::new();
    for m in g.modules.values() {
        let VisibilityScope::Private { root } = &m.visibility else {
            continue;
        };
        let Some(zone) = m
            .path
            .strip_prefix(root)
            .ok()
            .and_then(|rest| rest.components().next())
        else {
            continue;
        };
        let path = root.join(zone);
        zones
            .entry(path.clone())
            .or_insert_with(|| PrivateZone {
                path,
                root: root.clone(),
                files: vec![],
            })
            .files
            .push(m.id);
    }
    zones.into_values().collect()
}
