use std::collections::HashSet;

use globset::GlobSet;
use serde::Deserialize;
use stratum_config::Config;
use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::builtin::import_policy::{glob_set, project_relative};
use crate::ids::RESTRICTED_CALLS;
use crate::rule::Rule;

const SLUG: &str = "stratum/restricted-calls";
const DEFAULT_MESSAGE: &str = "Call restricted by restricted-calls";

#[derive(Debug, Default, Clone, Copy)]
pub struct RestrictedCalls;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(transparent)]
pub struct RestrictedCallsOptions(pub Vec<CallPolicy>);

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "CallPolicySpec")]
pub struct CallPolicy {
    pub calls: Vec<String>,
    allow: GlobSet,
    message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CallPolicySpec {
    calls: Vec<String>,
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    message: Option<String>,
}

impl TryFrom<CallPolicySpec> for CallPolicy {
    type Error = String;

    fn try_from(spec: CallPolicySpec) -> Result<Self, String> {
        Ok(Self {
            allow: glob_set(&spec.allow)?,
            calls: spec.calls,
            message: spec.message.unwrap_or_else(|| DEFAULT_MESSAGE.into()),
        })
    }
}

impl Rule for RestrictedCalls {
    type Scope = ModuleId;
    type Options = RestrictedCallsOptions;

    fn id(&self) -> RuleId {
        RESTRICTED_CALLS
    }
    fn slug(&self) -> &'static str {
        SLUG
    }
    fn default_severity(&self) -> Severity {
        Severity::Off
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &RestrictedCallsOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(module) = graph.modules.get(&scope) else {
            return Vec::new();
        };
        if module.calls.is_empty() || options.0.is_empty() {
            return Vec::new();
        }
        let path = project_relative(graph, module);
        module
            .calls
            .iter()
            .filter_map(|call| {
                let policy = options
                    .0
                    .iter()
                    .find(|p| p.calls.contains(&call.name) && !p.allow.is_match(&path))?;
                Some(Violation {
                    rule: self.id(),
                    severity,
                    message: format!("{} ({}())", policy.message, call.name),
                    file: module.path.clone(),
                    location: call.location,
                    modules: vec![scope],
                    edge: None,
                    suggestion: None,
                })
            })
            .collect()
    }
}

/// Names the parser must collect: every `calls` entry of `stratum/restricted-calls`, or none
/// when the rule is off in `rules` and in every override.
#[must_use]
pub fn tracked_calls(config: &Config) -> HashSet<String> {
    let Some(rule) = config.rules.get(SLUG) else {
        return HashSet::new();
    };
    let enabled = rule.severity != Severity::Off
        || config
            .overrides
            .iter()
            .filter_map(|o| o.rules.get(SLUG))
            .any(|o| o.severity != Severity::Off);
    if !enabled || rule.options.is_null() {
        return HashSet::new();
    }
    serde_json::from_value::<RestrictedCallsOptions>(rule.options.clone())
        .map(|opts| opts.0.into_iter().flat_map(|p| p.calls).collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
    use stratum_core::{
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::{Call, Exports, Layer, Module},
        violation::SourceLocation,
        visibility::VisibilityScope,
    };

    fn graph(paths: &[&str]) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
        g.layers.insert(
            LayerId::new(0),
            Layer {
                id: LayerId::new(0),
                name: "app".into(),
                path: PathBuf::from("src"),
                depends_on: vec![],
                slices: false,
            },
        );
        for (i, path) in paths.iter().enumerate() {
            let id = ModuleId::new(u32::try_from(i).unwrap());
            g.node_index.insert(id, g.deps.add_node(id));
            g.modules.insert(
                id,
                Module {
                    id,
                    path: PathBuf::from(format!("C:/work/web/{path}")),
                    container: ContainerId::new(0),
                    layer: LayerId::new(0),
                    purity: Purity::new(2).unwrap(),
                    visibility: VisibilityScope::Public,
                    exports: Exports::default(),
                    calls: vec![Call {
                        name: "inject".into(),
                        location: SourceLocation { line: 7, column: 3 },
                    }],
                },
            );
        }
        g
    }

    fn options() -> RestrictedCallsOptions {
        serde_json::from_value(serde_json::json!([{
            "calls": ["inject", "provide"],
            "allow": ["**/_shared/*-scope.ts"],
            "message": "Use useXxxScope()",
        }]))
        .unwrap()
    }

    #[test]
    fn call_in_an_allowed_file_passes_and_elsewhere_is_reported_at_its_line() {
        let g = graph(&[
            "src/screens/home/_shared/home-scope.ts",
            "src/screens/home/ui/Page.vue",
        ]);
        let opts = options();
        assert!(
            RestrictedCalls
                .check(&g, ModuleId::new(0), &opts, Severity::Error)
                .is_empty()
        );
        let v = RestrictedCalls.check(&g, ModuleId::new(1), &opts, Severity::Error);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].message, "Use useXxxScope() (inject())");
        assert_eq!(v[0].location, SourceLocation { line: 7, column: 3 });
    }

    #[test]
    fn bad_allow_glob_is_rejected_at_load() {
        let bad = serde_json::from_value::<RestrictedCallsOptions>(serde_json::json!([
            { "calls": ["inject"], "allow": ["a/{**"] }
        ]));
        assert!(bad.is_err());
    }
}
