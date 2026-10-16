use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::ids::MODULE_DOCS;
use crate::options::ModuleDocsOptions;
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct ModuleDocs;

impl Rule for ModuleDocs {
    type Scope = ProjectScope;
    type Options = ModuleDocsOptions;

    fn id(&self) -> RuleId {
        MODULE_DOCS
    }
    fn slug(&self) -> &'static str {
        "stratum/module-docs"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &ModuleDocsOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
