use petgraph::Direction;
use petgraph::visit::EdgeRef;

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
    visibility::VisibilityScope as VS,
};
use stratum_graph::CompoundGraph;

use crate::ids::VISIBILITY_SCOPE;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct VisibilityScope;

impl Rule for VisibilityScope {
    type Scope = ModuleId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        VISIBILITY_SCOPE
    }
    fn slug(&self) -> &'static str {
        "stratum/visibility-scope"
    }
    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(from) = graph.modules.get(&scope) else {
            return vec![];
        };
        let Some(node) = graph.node_for(scope) else {
            return vec![];
        };
        let mut out = Vec::new();
        for edge in graph.deps.edges_directed(node, Direction::Outgoing) {
            let to_id = graph.deps[edge.target()];
            let Some(to_mod) = graph.modules.get(&to_id) else {
                continue;
            };
            if !is_allowed(from, to_mod) {
                out.push(Violation {
                    rule: self.id(),
                    severity,
                    message: format!(
                        "Module is not allowed to import a module whose visibility excludes it ({:?})",
                        to_mod.visibility
                    ),
                    file: from.path.clone(),
                    location: SourceLocation { line: 1, column: 1 },
                    modules: vec![scope, to_id],
                    edge: Some(stratum_core::edge::Edge {
                        from: scope,
                        to: to_id,
                        kind: *edge.weight(),
                    }),
                    suggestion: Some(
                        "Either move the target module to a more permissive visibility, or move the importer into the allowed scope.".into(),
                    ),
                });
            }
        }
        out.sort_by(|a, b| {
            let am = a.modules.get(1).map_or(0, |m| m.raw());
            let bm = b.modules.get(1).map_or(0, |m| m.raw());
            am.cmp(&bm)
        });
        out
    }
}

fn is_allowed(from: &stratum_core::types::Module, to: &stratum_core::types::Module) -> bool {
    match &to.visibility {
        VS::Public => true,
        VS::Layer { id } => from.layer == *id,
        VS::Container { id } => from.container == *id,
        VS::Shared => from.path.to_string_lossy().contains("/_shared/"),
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
        stage::Stage,
        types::Module,
    };

    fn module(id: u32, layer: u32, container: u32, path: &str, visibility: VS) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(path),
            container: ContainerId::new(container),
            layer: LayerId::new(layer),
            stage: Stage::new(2).unwrap(),
            visibility,
        }
    }

    fn build(mods: Vec<Module>, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
        for m in mods {
            let node = g.deps.add_node(m.id);
            g.node_index.insert(m.id, node);
            g.modules.insert(m.id, m);
        }
        for (a, b) in edges {
            let na = g.node_index[&ModuleId::new(*a)];
            let nb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn public_target_is_allowed() {
        let g = build(
            vec![
                module(0, 1, 1, "src/a.ts", VS::Public),
                module(1, 2, 2, "src/b.ts", VS::Public),
            ],
            &[(0, 1)],
        );
        let v = VisibilityScope.check(&g, ModuleId::new(0), &EmptyOptions, Severity::Error);
        assert!(v.is_empty());
    }

    #[test]
    fn container_visibility_blocks_other_containers() {
        let g = build(
            vec![
                module(0, 1, 1, "src/a.ts", VS::Public),
                module(
                    1,
                    1,
                    2,
                    "src/b.ts",
                    VS::Container {
                        id: ContainerId::new(2),
                    },
                ),
            ],
            &[(0, 1)],
        );
        let v = VisibilityScope.check(&g, ModuleId::new(0), &EmptyOptions, Severity::Error);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn container_visibility_allows_same_container() {
        let g = build(
            vec![
                module(0, 1, 2, "src/a.ts", VS::Public),
                module(
                    1,
                    1,
                    2,
                    "src/b.ts",
                    VS::Container {
                        id: ContainerId::new(2),
                    },
                ),
            ],
            &[(0, 1)],
        );
        let v = VisibilityScope.check(&g, ModuleId::new(0), &EmptyOptions, Severity::Error);
        assert!(v.is_empty());
    }

    #[test]
    fn shared_visibility_allows_shared_sibling() {
        let g = build(
            vec![
                module(0, 1, 1, "src/features/_shared/a.ts", VS::Public),
                module(1, 1, 1, "src/features/cart/util.ts", VS::Shared),
            ],
            &[(0, 1)],
        );
        let v = VisibilityScope.check(&g, ModuleId::new(0), &EmptyOptions, Severity::Error);
        assert!(v.is_empty());
    }
}
