use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::ids::SIBLING_ISOLATION;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct SiblingIsolation;

impl Rule for SiblingIsolation {
    type Scope = ModuleId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        SIBLING_ISOLATION
    }
    fn slug(&self) -> &'static str {
        "stratum/sibling-isolation"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ModuleId,
        _options: &EmptyOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
