use petgraph::Direction;
use petgraph::visit::EdgeRef;

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::PURITY;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct PurityRule;

impl Rule for PurityRule {
    type Scope = ModuleId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        PURITY
    }
    fn slug(&self) -> &'static str {
        "stratum/purity"
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
        let Some(node) = graph.node_for(scope) else {
            return vec![];
        };
        let mut out = Vec::new();
        for edge in graph.deps.edges_directed(node, Direction::Outgoing) {
            let to_id = graph.deps[edge.target()];
            let Some(to_mod) = graph.modules.get(&to_id) else {
                continue;
            };
            if !from_mod.purity.may_depend_on(to_mod.purity) {
                out.push(Violation {
                    rule: self.id(),
                    severity,
                    message: format!(
                        "Module at purity {} imports from purity {}: purer code cannot depend on impurer code",
                        from_mod.purity.rank(),
                        to_mod.purity.rank()
                    ),
                    file: from_mod.path.clone(),
                    location: SourceLocation { line: 1, column: 1 },
                    modules: vec![scope, to_id],
                    edge: Some(stratum_core::edge::Edge {
                        from: scope,
                        to: to_id,
                        kind: *edge.weight(),
                    }),
                    suggestion: Some(
                        "Move shared logic into a module at or below the lower purity, or inject the impure dependency at composition root."
                            .into(),
                    ),
                });
            }
        }
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
        types::Module,
        visibility::VisibilityScope,
    };

    fn module_at_purity(id: u32, purity_rank: u8) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("src/m{id}.ts")),
            container: ContainerId::new(0),
            layer: LayerId::new(0),
            purity: Purity::new(purity_rank).unwrap(),
            visibility: VisibilityScope::Public,
            exports: stratum_core::types::Exports::default(),
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
    fn impurer_can_depend_on_purer() {
        let g = build(
            vec![module_at_purity(0, 4), module_at_purity(1, 1)],
            &[(0, 1)],
        );
        let viols = PurityRule.check(&g, ModuleId::new(0), &EmptyOptions, Severity::Error);
        assert!(viols.is_empty());
    }

    #[test]
    fn purer_cannot_depend_on_impurer() {
        let g = build(
            vec![module_at_purity(0, 1), module_at_purity(1, 4)],
            &[(0, 1)],
        );
        let viols = PurityRule.check(&g, ModuleId::new(0), &EmptyOptions, Severity::Error);
        assert_eq!(viols.len(), 1);
        assert_eq!(viols[0].rule, PURITY);
    }

    #[test]
    fn same_purity_dep_allowed() {
        let g = build(
            vec![module_at_purity(0, 2), module_at_purity(1, 2)],
            &[(0, 1)],
        );
        let viols = PurityRule.check(&g, ModuleId::new(0), &EmptyOptions, Severity::Error);
        assert!(viols.is_empty());
    }
}
