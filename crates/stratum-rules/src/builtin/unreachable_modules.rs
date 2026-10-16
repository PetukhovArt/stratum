use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::ids::UNREACHABLE_MODULES;
use crate::rule::{EmptyOptions, Rule};
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct UnreachableModules;

impl Rule for UnreachableModules {
    type Scope = ProjectScope;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        UNREACHABLE_MODULES
    }
    fn slug(&self) -> &'static str {
        "stratum/unreachable-modules"
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
