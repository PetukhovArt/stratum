use std::hash::Hash;

use stratum_core::ids::{ContainerId, ModuleId};
use stratum_graph::CompoundGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectScope;

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
