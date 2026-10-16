use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::ids::MISSING_ENTRY;
use crate::rule::{EmptyOptions, Rule};
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct MissingEntry;

impl Rule for MissingEntry {
    type Scope = ProjectScope;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        MISSING_ENTRY
    }
    fn slug(&self) -> &'static str {
        "stratum/missing-entry"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
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
