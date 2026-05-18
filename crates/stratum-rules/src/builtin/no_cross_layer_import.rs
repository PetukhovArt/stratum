use std::collections::HashSet;

use petgraph::Direction;
use petgraph::visit::EdgeRef;

use stratum_core::{
    ids::{LayerId, ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::NO_CROSS_LAYER_IMPORT;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct NoCrossLayerImport;

impl Rule for NoCrossLayerImport {
    type Scope = ModuleId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        NO_CROSS_LAYER_IMPORT
    }
    fn slug(&self) -> &'static str {
        "stratum/no-cross-layer-import"
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
        let Some(from_mod) = graph.modules.get(&scope) else {
            return vec![];
        };
        let from_layer = from_mod.layer;
        let Some(layer_def) = graph.layers.get(&from_layer) else {
            return vec![];
        };
        let allowed: HashSet<LayerId> = layer_def
            .depends_on
            .iter()
            .copied()
            .chain(std::iter::once(from_layer))
            .collect();
        let Some(node) = graph.node_for(scope) else {
            return vec![];
        };
        let mut out = Vec::new();
        for edge in graph.deps.edges_directed(node, Direction::Outgoing) {
            let to_id = graph.deps[edge.target()];
            let Some(to_mod) = graph.modules.get(&to_id) else {
                continue;
            };
            if !allowed.contains(&to_mod.layer) {
                let from_layer_name = layer_def.name.clone();
                let to_layer_name = graph.layers.get(&to_mod.layer).map_or_else(
                    || format!("layer#{}", to_mod.layer.raw()),
                    |l| l.name.clone(),
                );
                out.push(Violation {
                    rule: self.id(),
                    severity,
                    message: format!(
                        "Module imports from layer '{to_layer_name}' which '{from_layer_name}' does not depend on"
                    ),
                    file: from_mod.path.clone(),
                    location: SourceLocation { line: 1, column: 1 },
                    modules: vec![scope, to_id],
                    edge: Some(stratum_core::edge::Edge {
                        from: scope,
                        to: to_id,
                        kind: *edge.weight(),
                    }),
                    suggestion: Some(format!(
                        "Move the imported symbol into a layer '{from_layer_name}' may reach, or add '{to_layer_name}' to '{from_layer_name}'.depends_on if the dependency is legitimate."
                    )),
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
        types::{Layer, Module},
        visibility::VisibilityScope,
    };

    fn module(id: u32, layer_raw: u32, path: &str) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(path),
            container: ContainerId::new(layer_raw),
            layer: LayerId::new(layer_raw),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
        }
    }

    fn build_graph(
        layers: &[(u32, &str, Vec<u32>)],
        modules: Vec<Module>,
        edges: &[(u32, u32)],
    ) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
        for (id, name, deps) in layers {
            g.layers.insert(
                LayerId::new(*id),
                Layer {
                    id: LayerId::new(*id),
                    name: (*name).into(),
                    path: PathBuf::from(format!("src/{name}")),
                    depends_on: deps.iter().map(|i| LayerId::new(*i)).collect(),
                },
            );
        }
        for m in modules {
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
    fn allowed_dep_is_not_flagged() {
        let g = build_graph(
            &[(1, "features", vec![2]), (2, "shared", vec![])],
            vec![
                module(10, 1, "src/features/cart.ts"),
                module(20, 2, "src/shared/http.ts"),
            ],
            &[(10, 20)],
        );
        let viols = NoCrossLayerImport.check(&g, ModuleId::new(10), &EmptyOptions, Severity::Error);
        assert!(viols.is_empty());
    }

    #[test]
    fn upward_dep_is_flagged() {
        let g = build_graph(
            &[(1, "features", vec![2]), (2, "shared", vec![])],
            vec![
                module(10, 1, "src/features/cart.ts"),
                module(20, 2, "src/shared/http.ts"),
            ],
            &[(20, 10)],
        );
        let viols = NoCrossLayerImport.check(&g, ModuleId::new(20), &EmptyOptions, Severity::Error);
        assert_eq!(viols.len(), 1);
        assert_eq!(viols[0].rule, NO_CROSS_LAYER_IMPORT);
        assert_eq!(viols[0].modules, vec![ModuleId::new(20), ModuleId::new(10)]);
    }

    #[test]
    fn self_layer_dep_is_allowed() {
        let g = build_graph(
            &[(1, "features", vec![])],
            vec![
                module(10, 1, "src/features/a.ts"),
                module(11, 1, "src/features/b.ts"),
            ],
            &[(10, 11)],
        );
        let viols = NoCrossLayerImport.check(&g, ModuleId::new(10), &EmptyOptions, Severity::Error);
        assert!(viols.is_empty());
    }
}
