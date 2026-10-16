use stratum_core::{
    ids::RuleId,
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, ownership};

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
        graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        ownership::unreachable_modules(graph)
            .into_iter()
            .filter_map(|id| graph.modules.get(&id))
            .map(|m| Violation {
                rule: self.id(),
                severity,
                message: "No entry point reaches this module through any import".into(),
                file: m.path.clone(),
                location: SourceLocation { line: 1, column: 1 },
                modules: vec![m.id],
                edge: None,
                fix: None,
            })
            .collect()
    }
}
