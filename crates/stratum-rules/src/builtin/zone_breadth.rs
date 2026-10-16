use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use super::folder::{at_module, relative};
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
        graph: &CompoundGraph,
        scope: ContainerId,
        options: &ZoneBreadthOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(c) = graph.containers.get(&scope) else {
            return vec![];
        };
        let max = usize::try_from(options.max_kinds).unwrap_or(usize::MAX);
        graph
            .private_zones_in(scope)
            .filter_map(|zone| {
                let kinds = graph.kinds(&zone.path).len();
                if kinds <= max {
                    return None;
                }
                let name = relative(&zone.path, &c.path);
                at_module(
                    graph,
                    ZONE_BREADTH,
                    severity,
                    *zone.files.first()?,
                    format!(
                        "Private zone '{name}' of '{owner}' holds {kinds} kinds directly inside it (limit {max})",
                        owner = c.name,
                        max = options.max_kinds,
                    ),
                    format!(
                        "Group the files of '{name}' into folders by purpose, or promote a cohesive group into a sub-unit with its own entry file."
                    ),
                )
            })
            .collect()
    }
}
