use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::ids::SUPPRESSION;
use crate::options::SuppressionOptions;
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct Suppression;

impl Rule for Suppression {
    type Scope = ProjectScope;
    type Options = SuppressionOptions;

    fn id(&self) -> RuleId {
        SUPPRESSION
    }
    fn slug(&self) -> &'static str {
        "stratum/suppression"
    }
    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &SuppressionOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
