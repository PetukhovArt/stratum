use petgraph::Direction;

use stratum_core::{
    ids::{ContainerId, ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::DEPTH_RATIO;
use crate::options::DepthRatioOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct DepthRatio;

impl Rule for DepthRatio {
    type Scope = ModuleId;
    type Options = DepthRatioOptions;

    fn id(&self) -> RuleId {
        DEPTH_RATIO
    }
    fn slug(&self) -> &'static str {
        "stratum/depth-ratio"
    }
    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &DepthRatioOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(m) = graph.modules.get(&scope) else {
            return vec![];
        };
        let container = m.container;
        let internal = container_size(graph, container);
        let Some(node) = graph.node_for(scope) else {
            return vec![];
        };
        let outgoing = graph
            .deps
            .edges_directed(node, Direction::Outgoing)
            .count();
        if outgoing == 0 {
            return vec![];
        }
        #[allow(clippy::cast_precision_loss)]
        let ratio = internal as f64 / outgoing as f64;
        if ratio >= options.min_ratio {
            return vec![];
        }
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Depth ratio {ratio:.2} is below the configured minimum {min:.2}; module exposes too much relative to its container size",
                min = options.min_ratio
            ),
            file: m.path.clone(),
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![scope],
            edge: None,
            suggestion: Some(
                "Consolidate functionality so each public symbol stands for more internal work, or split the container.".into(),
            ),
        }]
    }
}

fn container_size(g: &CompoundGraph, c: ContainerId) -> usize {
    g.modules.values().filter(|m| m.container == c).count()
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
            path: PathBuf::from(format!("src/m{id}.ts")),
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
    fn no_outgoing_no_violation() {
        let g = build(vec![module(0, 1)], &[]);
        let viols = DepthRatio.check(
            &g,
            ModuleId::new(0),
            &DepthRatioOptions { min_ratio: 3.0 },
            Severity::Info,
        );
        assert!(viols.is_empty());
    }

    #[test]
    fn high_ratio_no_violation() {
        let g = build(
            vec![module(0, 1), module(1, 1), module(2, 1), module(3, 1)],
            &[(0, 1)],
        );
        let viols = DepthRatio.check(
            &g,
            ModuleId::new(0),
            &DepthRatioOptions { min_ratio: 3.0 },
            Severity::Info,
        );
        assert!(viols.is_empty());
    }

    #[test]
    fn low_ratio_emits_violation() {
        let g = build(
            vec![module(0, 1), module(1, 1), module(2, 2), module(3, 2)],
            &[(0, 2), (0, 3)],
        );
        let viols = DepthRatio.check(
            &g,
            ModuleId::new(0),
            &DepthRatioOptions { min_ratio: 3.0 },
            Severity::Info,
        );
        assert_eq!(viols.len(), 1);
        assert_eq!(viols[0].rule, DEPTH_RATIO);
    }
}
