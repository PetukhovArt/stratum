use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::ids::CONTAINER_BREADTH;
use crate::options::ContainerBreadthOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct ContainerBreadth;

impl Rule for ContainerBreadth {
    type Scope = ContainerId;
    type Options = ContainerBreadthOptions;

    fn id(&self) -> RuleId {
        CONTAINER_BREADTH
    }
    fn slug(&self) -> &'static str {
        "stratum/container-breadth"
    }
    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ContainerId,
        _options: &ContainerBreadthOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
