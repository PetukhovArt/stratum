use stratum_core::text::{Key, Text};
use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::{CompoundGraph, ownership};

use crate::ids::UNUSED_EXPORTS;
use crate::options::UnusedExportsOptions;
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct UnusedExports;

impl Rule for UnusedExports {
    type Scope = ProjectScope;
    type Options = UnusedExportsOptions;

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
        options: &UnusedExportsOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        ownership::unused_exports(graph, options.count_ignored_importers)
            .into_iter()
            .filter(|(_, decl)| !(options.ignore_type_exports && decl.type_only))
            .filter_map(|(id, decl)| Some((graph.modules.get(&id)?, decl)))
            .map(|(m, decl)| {
                let message = Text::new(Key::UnusedExport).arg("name", &decl.name);
                Violation {
                    rule: self.id(),
                    severity,
                    message: message.en(),
                    file: m.path.clone(),
                    location: decl.location,
                    modules: vec![m.id],
                    edge: None,
                    fix: None,
                    message_text: Some(message),
                    fix_text: None,
                }
            })
            .collect()
    }
}
