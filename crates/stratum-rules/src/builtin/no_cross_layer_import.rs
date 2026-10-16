use stratum_core::{
    ids::{LayerId, ModuleId, RuleId},
    severity::Severity,
    types::Layer,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::builtin::edge::{EdgeJudge, Finding, edge_violations};
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

    fn edge_judge(&self) -> Option<EdgeJudge<EmptyOptions>> {
        Some(EdgeJudge(|graph, from, to, _| judge(graph, from, to)))
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        edge_violations(graph, scope, self.id(), severity, |to| {
            judge(graph, scope, to)
        })
    }
}

pub(crate) fn may_import(from: &Layer, to: LayerId) -> bool {
    from.id == to || from.depends_on.contains(&to)
}

pub(crate) fn judge(graph: &CompoundGraph, from: ModuleId, to: ModuleId) -> Option<Finding> {
    let from_layer = graph.layers.get(&graph.modules.get(&from)?.layer)?;
    let to_layer = graph.modules.get(&to)?.layer;
    if may_import(from_layer, to_layer) {
        return None;
    }
    let from_layer_name = &from_layer.name;
    let to_layer_name = graph
        .layers
        .get(&to_layer)
        .map_or_else(|| format!("layer#{}", to_layer.raw()), |l| l.name.clone());
    Some(Finding {
        message: format!(
            "Module imports from layer '{to_layer_name}' which '{from_layer_name}' does not depend on"
        ),
        suggestion: Some(format!(
            "Move the imported symbol into a layer '{from_layer_name}' may reach, or add '{to_layer_name}' to '{from_layer_name}'.depends_on if the dependency is legitimate."
        )),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::{Layer, Module},
        visibility::VisibilityScope,
    };

    fn module(id: u32, layer_raw: u32, path: &str) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(path),
            container: ContainerId::new(layer_raw),
            layer: LayerId::new(layer_raw),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: stratum_core::types::Exports::default(),
            calls: vec![],
        }
    }

    fn build_graph(
        layers: &[(u32, &str, Vec<u32>)],
        modules: Vec<Module>,
        edges: &[(u32, u32)],
    ) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for (id, name, deps) in layers {
            g.layers.insert(
                LayerId::new(*id),
                Layer {
                    id: LayerId::new(*id),
                    name: (*name).into(),
                    path: PathBuf::from(format!("src/{name}")),
                    depends_on: deps.iter().map(|i| LayerId::new(*i)).collect(),
                    slices: false,
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
