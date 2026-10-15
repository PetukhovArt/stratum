use indexmap::IndexMap;
use petgraph::graph::{DiGraph, NodeIndex};

use stratum_core::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId},
    types::{Container, Layer, Module},
};

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
    pub layers: IndexMap<LayerId, Layer>,
    pub deps: DiGraph<ModuleId, EdgeKind>,
    pub node_index: rustc_hash::FxHashMap<ModuleId, NodeIndex>,
}

impl CompoundGraph {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: rustc_hash::FxHashMap::default(),
        }
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
}

impl Default for CompoundGraph {
    fn default() -> Self {
        Self::empty()
    }
}
