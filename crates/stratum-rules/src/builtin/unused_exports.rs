use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::{CompoundGraph, ownership};

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
        graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        ownership::unused_exports(graph)
            .into_iter()
            .filter_map(|(id, decl)| Some((graph.modules.get(&id)?, decl)))
            .map(|(m, decl)| Violation {
                rule: self.id(),
                severity,
                message: format!(
                    "Export `{}` is imported by no module reachable from the entry points",
                    decl.name
                ),
                file: m.path.clone(),
                location: decl.location,
                modules: vec![m.id],
                edge: None,
                fix: None,
            })
            .collect()
    }
}
