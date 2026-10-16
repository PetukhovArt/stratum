use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::ids::ZONE_BREADTH;
use crate::options::ZoneBreadthOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct ZoneBreadth;

impl Rule for ZoneBreadth {
    type Scope = ContainerId;
    type Options = ZoneBreadthOptions;

    fn id(&self) -> RuleId {
        ZONE_BREADTH
    }
    fn slug(&self) -> &'static str {
        "stratum/zone-breadth"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ContainerId,
        _options: &ZoneBreadthOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
