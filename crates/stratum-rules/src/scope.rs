use std::hash::Hash;

use stratum_core::ids::{ContainerId, ModuleId};
use stratum_graph::CompoundGraph;

/// Marker for the project-wide scope; rules that scope to the whole graph
/// take this as their `Scope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectScope;

/// The set of inputs a `Rule` is keyed on.
pub trait RuleScope: Copy + Eq + Hash + std::fmt::Debug + Send + Sync + 'static {
    fn enumerate(graph: &CompoundGraph) -> Vec<Self>;
}

impl RuleScope for ModuleId {
    fn enumerate(graph: &CompoundGraph) -> Vec<Self> {
        graph.modules.keys().copied().collect()
    }
}

impl RuleScope for ContainerId {
    fn enumerate(graph: &CompoundGraph) -> Vec<Self> {
        graph.containers.keys().copied().collect()
    }
}

impl RuleScope for ProjectScope {
    fn enumerate(_graph: &CompoundGraph) -> Vec<Self> {
        vec![ProjectScope]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_scope_singleton() {
        let g = CompoundGraph::empty();
        assert_eq!(ProjectScope::enumerate(&g), vec![ProjectScope]);
    }

    #[test]
    fn module_scope_enumerates_all_modules() {
        let g = CompoundGraph::empty();
        assert!(ModuleId::enumerate(&g).is_empty());
    }
}
