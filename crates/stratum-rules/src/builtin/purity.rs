use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::builtin::edge::{EdgeJudge, Finding, edge_violations};
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

pub(crate) fn judge(graph: &CompoundGraph, from: ModuleId, to: ModuleId) -> Option<Finding> {
    let from = graph.modules.get(&from)?.purity;
    let to = graph.modules.get(&to)?.purity;
    if from.may_depend_on(to) {
        return None;
    }
    Some(Finding {
        message: format!(
            "Module at purity {} imports from purity {}: purer code cannot depend on impurer code",
            from.rank(),
            to.rank()
        ),
        suggestion: Some(
            "Move shared logic into a module at or below the lower purity, or inject the impure dependency at composition root."
                .into(),
        ),
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
            calls: vec![],
        }
    }

    fn build(mods: Vec<Module>, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
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
