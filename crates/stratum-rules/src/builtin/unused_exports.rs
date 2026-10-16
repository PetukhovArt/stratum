use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::ids::UNUSED_EXPORTS;
use crate::rule::{EmptyOptions, Rule};
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct UnusedExports;

impl Rule for UnusedExports {
    type Scope = ProjectScope;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        UNUSED_EXPORTS
    }
    fn slug(&self) -> &'static str {
        "stratum/unused-exports"
    }
    fn default_severity(&self) -> Severity {
        Severity::Off
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &EmptyOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
