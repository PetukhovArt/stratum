use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::ids::NAMING;
use crate::options::NamingOptions;
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct Naming;

impl Rule for Naming {
    type Scope = ProjectScope;
    type Options = NamingOptions;

    fn id(&self) -> RuleId {
        NAMING
    }
    fn slug(&self) -> &'static str {
        "stratum/naming"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &NamingOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
