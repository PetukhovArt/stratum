use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::ids::STAGE_MIXING;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct StageMixing;

impl Rule for StageMixing {
    type Scope = ContainerId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        STAGE_MIXING
    }
    fn slug(&self) -> &'static str {
        "stratum/stage-mixing"
    }
    fn default_severity(&self) -> Severity {
        Severity::Off
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ContainerId,
        _options: &EmptyOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
