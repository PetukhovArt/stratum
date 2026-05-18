use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, metrics::promotion_pressure};

use crate::ids::PROMOTION_PRESSURE;
use crate::options::PromotionPressureOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct PromotionPressure;

impl Rule for PromotionPressure {
    type Scope = ModuleId;
    type Options = PromotionPressureOptions;

    fn id(&self) -> RuleId {
        PROMOTION_PRESSURE
    }
    fn slug(&self) -> &'static str {
        "stratum/promotion-pressure"
    }
    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &PromotionPressureOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(m) = graph.modules.get(&scope) else {
            return vec![];
        };
        let pressure = promotion_pressure(graph, scope);
        if pressure < options.threshold {
            return vec![];
        }
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Module is imported across containers by {pressure} dependents (threshold {limit}); consider promoting it to a lower layer",
                limit = options.threshold
            ),
            file: m.path.clone(),
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![scope],
            edge: None,
            suggestion: Some(
                "Move this module into the lowest layer that every dependent already depends on."
                    .into(),
            ),
        }]
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
        visibility::VisibilityScope,
    };

    fn module(id: u32, container: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(container),
            layer: LayerId::new(0),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
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
    fn under_threshold_no_violation() {
        let g = build(
            vec![module(0, 1), module(1, 2), module(2, 2)],
            &[(1, 0), (2, 0)],
        );
        let v = PromotionPressure.check(
            &g,
            ModuleId::new(0),
            &PromotionPressureOptions { threshold: 5 },
            Severity::Info,
        );
        assert!(v.is_empty());
    }

    #[test]
    fn over_threshold_emits_violation() {
        let mut mods = vec![module(0, 1)];
        for i in 1..=6u32 {
            mods.push(module(i, 2));
        }
        let edges: Vec<(u32, u32)> = (1..=6).map(|i| (i, 0)).collect();
        let g = build(mods, &edges);
        let v = PromotionPressure.check(
            &g,
            ModuleId::new(0),
            &PromotionPressureOptions { threshold: 5 },
            Severity::Info,
        );
        assert_eq!(v.len(), 1);
    }
}
