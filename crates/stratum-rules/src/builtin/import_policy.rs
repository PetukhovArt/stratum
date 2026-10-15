use std::collections::BTreeSet;

use globset::{GlobBuilder, GlobMatcher, GlobSet, GlobSetBuilder};
use petgraph::Direction;
use petgraph::visit::EdgeRef;
use serde::Deserialize;

use stratum_core::{
    edge::Edge,
    ids::{ModuleId, RuleId},
    severity::Severity,
    types::Module,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::docs::RuleDoc;
use crate::ids::IMPORT_POLICY;
use crate::rule::Rule;

const DEFAULT_MESSAGE: &str = "Import denied by import-policy";

#[derive(Debug, Default, Clone, Copy)]
pub struct ImportPolicy;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(transparent)]
pub struct ImportPolicyOptions(pub Vec<Policy>);

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "PolicySpec")]
pub struct Policy {
    from: GlobSet,
    except_from: GlobSet,
    deny: GlobSet,
    allow: GlobSet,
    slice: Option<GlobMatcher>,
    message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicySpec {
    from: Vec<String>,
    #[serde(default)]
    except_from: Vec<String>,
    deny: Vec<String>,
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    slice: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

impl TryFrom<PolicySpec> for Policy {
    type Error = String;

    fn try_from(spec: PolicySpec) -> Result<Self, String> {
        Ok(Self {
            from: glob_set(&spec.from)?,
            except_from: glob_set(&spec.except_from)?,
            deny: glob_set(&spec.deny)?,
            allow: glob_set(&spec.allow)?,
            slice: spec
                .slice
                .as_deref()
                .map(|p| glob(p).map(|g| g.compile_matcher()))
                .transpose()?,
            message: spec.message.unwrap_or_else(|| DEFAULT_MESSAGE.into()),
        })
    }
}

fn glob(pattern: &str) -> Result<globset::Glob, String> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map_err(|e| format!("invalid glob {pattern:?}: {e}"))
}

fn glob_set(patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(glob(p)?);
    }
    builder.build().map_err(|e| e.to_string())
}

impl Policy {
    fn denies(&self, from: &str, to: &str) -> bool {
        self.from.is_match(from)
            && !self.except_from.is_match(from)
            && self.deny.is_match(to)
            && !self.allow.is_match(to)
            && !self.same_slice(from, to)
    }

    fn same_slice(&self, from: &str, to: &str) -> bool {
        self.slice.as_ref().is_some_and(|slice| {
            let root = slice_root(slice, from);
            root.is_some() && root == slice_root(slice, to)
        })
    }
}

fn slice_root<'a>(slice: &GlobMatcher, path: &'a str) -> Option<&'a str> {
    path.match_indices('/')
        .map(|(i, _)| &path[..i])
        .find(|dir| slice.is_match(dir))
}

fn project_relative(graph: &CompoundGraph, module: &Module) -> String {
    let path = module.path.to_string_lossy().replace('\\', "/");
    let Some(layer) = graph.layers.get(&module.layer) else {
        return path;
    };
    let layer_path = layer.path.to_string_lossy().replace('\\', "/");
    let layer_path = layer_path.trim_start_matches("./").trim_matches('/');
    if path.starts_with(&format!("{layer_path}/")) {
        return path;
    }
    path.find(&format!("/{layer_path}/"))
        .map_or(path.clone(), |i| path[i + 1..].to_owned())
}

impl Rule for ImportPolicy {
    type Scope = ModuleId;
    type Options = ImportPolicyOptions;

    fn id(&self) -> RuleId {
        IMPORT_POLICY
    }
    fn slug(&self) -> &'static str {
        "stratum/import-policy"
    }
    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn doc(&self) -> Option<RuleDoc> {
        Some(RuleDoc {
            summary: "Denies imports between path globs declared in config, without scripting."
                .into(),
            why: "Layers say which layer may import which; import-policy covers the finer boundaries: slices that must not reach into each other, private folders reachable only through an index, zones a team has closed.".into(),
            fix: "Import through the allowed entry point (often the slice's index), move the shared code down into a layer both sides may import, or change the policy if the boundary moved on purpose.".into(),
            options: vec![
                "options: a list of policies; an import is reported by the first policy that denies it".into(),
                "from: globs of importing files, relative to the project root (required)".into(),
                "except_from: importing files the policy skips ([])".into(),
                "deny: globs of imported files (required)".into(),
                "allow: imported files exempt from deny, e.g. a slice's index.ts ([])".into(),
                "slice: glob of slice directories, e.g. src/features/*; imports inside one slice are allowed (none)".into(),
                format!("message: text of the violation; the imported path is appended (\"{DEFAULT_MESSAGE}\")"),
            ],
        })
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &ImportPolicyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        if options.0.is_empty() {
            return Vec::new();
        }
        let (Some(from_mod), Some(node)) = (graph.modules.get(&scope), graph.node_for(scope))
        else {
            return Vec::new();
        };
        let from = project_relative(graph, from_mod);
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for edge in graph.deps.edges_directed(node, Direction::Outgoing) {
            let to_id = graph.deps[edge.target()];
            let Some(to_mod) = graph.modules.get(&to_id) else {
                continue;
            };
            if !seen.insert(to_id) {
                continue;
            }
            let to = project_relative(graph, to_mod);
            let Some(policy) = options.0.iter().find(|p| p.denies(&from, &to)) else {
                continue;
            };
            out.push(Violation {
                rule: self.id(),
                severity,
                message: format!("{} ({to})", policy.message),
                file: from_mod.path.clone(),
                location: SourceLocation { line: 1, column: 1 },
                modules: vec![scope, to_id],
                edge: Some(Edge {
                    from: scope,
                    to: to_id,
                    kind: *edge.weight(),
                }),
                suggestion: None,
            });
        }
        out.sort_by_key(|v| v.modules.get(1).map_or(0, |m| m.raw()));
        out
    }
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
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::Layer,
        visibility::VisibilityScope,
    };

    fn graph(paths: &[&str], edges: &[(u32, u32)]) -> CompoundGraph {
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
                name: "renderer".into(),
                path: PathBuf::from("src/renderer"),
                depends_on: vec![],
                slices: false,
            },
        );
        for (i, path) in paths.iter().enumerate() {
            let id = ModuleId::new(u32::try_from(i).unwrap());
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
            g.modules.insert(
                id,
                Module {
                    id,
                    path: PathBuf::from(format!("C:/work/web/{path}")),
                    container: ContainerId::new(0),
                    layer: LayerId::new(0),
                    purity: Purity::new(2).unwrap(),
                    visibility: VisibilityScope::Public,
                    exports: stratum_core::types::Exports::default(),
                    calls: vec![],
                },
            );
        }
        for (a, b) in edges {
            let (na, nb) = (
                g.node_index[&ModuleId::new(*a)],
                g.node_index[&ModuleId::new(*b)],
            );
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn cross_slice_import_is_denied_while_same_slice_and_allowed_entry_pass() {
        let g = graph(
            &[
                "src/renderer/features/auth/ui/Login.vue",
                "src/renderer/features/auth/model/session.ts",
                "src/renderer/features/cart/model/cart.ts",
                "src/renderer/features/cart/index.ts",
            ],
            &[(0, 1), (0, 2), (0, 3)],
        );
        let options: ImportPolicyOptions = serde_json::from_value(serde_json::json!([{
            "from": ["src/renderer/features/**"],
            "deny": ["src/renderer/features/**"],
            "allow": ["src/renderer/features/*/index.ts"],
            "slice": "src/renderer/features/*",
            "message": "A feature does not import another feature",
        }]))
        .unwrap();
        let v = ImportPolicy.check(&g, ModuleId::new(0), &options, Severity::Error);
        assert_eq!(v.len(), 1, "{v:?}");
        assert_eq!(
            v[0].message,
            "A feature does not import another feature (src/renderer/features/cart/model/cart.ts)"
        );
    }

    #[test]
    fn unknown_policy_key_and_bad_glob_are_rejected() {
        let typo = serde_json::from_value::<ImportPolicyOptions>(serde_json::json!([
            { "from": ["a/**"], "deny": ["b/**"], "excpet": ["c"] }
        ]));
        assert!(typo.unwrap_err().to_string().contains("excpet"));
        let glob = serde_json::from_value::<ImportPolicyOptions>(serde_json::json!([
            { "from": ["a/{**"], "deny": ["b/**"] }
        ]));
        assert!(glob.is_err());
    }
}
