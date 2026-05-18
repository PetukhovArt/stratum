use std::path::PathBuf;
use std::sync::Arc;

use rhai::{AST, Array, Engine, Scope};

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;
use stratum_rules::rule::{EmptyOptions, Rule};

#[derive(Debug)]
pub struct RhaiRule {
    pub id: RuleId,
    pub slug: &'static str,
    pub ast: AST,
    pub engine: Arc<Engine>,
}

impl RhaiRule {
    fn invoke(&self, view: &crate::module_view::ModuleView) -> Option<Array> {
        let mut scope = Scope::new();
        let result: rhai::Dynamic =
            match self
                .engine
                .call_fn(&mut scope, &self.ast, "check", (view.clone(),))
            {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[rhai-rule {}] call_fn failed: {e}", self.slug);
                    return None;
                }
            };
        result.into_array().ok()
    }
}

impl Rule for RhaiRule {
    type Scope = ModuleId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        self.id
    }

    fn slug(&self) -> &'static str {
        self.slug
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(view) = crate::module_view::build_view(graph, scope) else {
            return Vec::new();
        };
        let Some(arr) = self.invoke(&view) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in arr {
            let Some(map) = entry.try_cast::<rhai::Map>() else {
                continue;
            };
            let message = map
                .get("message")
                .and_then(|d| d.clone().into_string().ok())
                .unwrap_or_default();
            let line = map
                .get("line")
                .and_then(|d| d.as_int().ok())
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(1);
            let suggestion = map
                .get("suggestion")
                .and_then(|d| d.clone().into_string().ok());
            let file = graph
                .modules
                .get(&scope)
                .map_or_else(PathBuf::new, |m| m.path.clone());
            out.push(Violation {
                rule: self.id,
                severity,
                message,
                file,
                location: SourceLocation { line, column: 1 },
                modules: vec![scope],
                edge: None,
                suggestion,
            });
        }
        out
    }
}
