use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::ids::NO_BARREL;
use crate::options::NoBarrelOptions;
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoBarrel;

impl Rule for NoBarrel {
    type Scope = ProjectScope;
    type Options = NoBarrelOptions;

    fn id(&self) -> RuleId {
        NO_BARREL
    }
    fn slug(&self) -> &'static str {
        "stratum/no-barrel"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &NoBarrelOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
