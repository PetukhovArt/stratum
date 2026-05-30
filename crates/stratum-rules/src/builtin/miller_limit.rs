use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, metrics::miller_fanout};

use crate::ids::MILLER_LIMIT;
use crate::options::MillerLimitOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct MillerLimit;

impl Rule for MillerLimit {
    type Scope = ModuleId;
    type Options = MillerLimitOptions;

    fn id(&self) -> RuleId {
        MILLER_LIMIT
    }
    fn slug(&self) -> &'static str {
        "stratum/miller-limit"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &MillerLimitOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(m) = graph.modules.get(&scope) else {
            return vec![];
        };
        let fanout = miller_fanout(graph, scope);
        let fanout_u32: u32 = u32::try_from(fanout).unwrap_or(u32::MAX);
        if fanout_u32 <= options.max_children {
            return vec![];
        }
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Cross-container fan-out {fanout} exceeds the configured limit {limit}",
                limit = options.max_children
            ),
            file: m.path.clone(),
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![scope],
            edge: None,
            suggestion: Some(
                "Split the module so each consumer reaches only the surface it needs, or move the implementation closer to its consumers.".into(),
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
        purity::Purity,
        types::Module,
        visibility::VisibilityScope,
    };

    fn module(id: u32, container: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("src/m{id}.ts")),
            container: ContainerId::new(container),
            layer: LayerId::new(0),
            purity: Purity::new(2).unwrap(),
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
    fn under_limit_no_violation() {
        let target = module(0, 1);
        let mut mods = vec![target];
        for i in 1..=3u32 {
            mods.push(module(i, 2));
        }
        let edges: Vec<(u32, u32)> = (1..=3).map(|i| (i, 0)).collect();
        let g = build(mods, &edges);
        let viols = MillerLimit.check(
            &g,
            ModuleId::new(0),
            &MillerLimitOptions { max_children: 7 },
            Severity::Warning,
        );
        assert!(viols.is_empty());
    }

    #[test]
    fn over_limit_emits_violation() {
        let target = module(0, 1);
        let mut mods = vec![target];
        for i in 1..=8u32 {
            mods.push(module(i, 2));
        }
        let edges: Vec<(u32, u32)> = (1..=8).map(|i| (i, 0)).collect();
        let g = build(mods, &edges);
        let viols = MillerLimit.check(
            &g,
            ModuleId::new(0),
            &MillerLimitOptions { max_children: 7 },
            Severity::Warning,
        );
        assert_eq!(viols.len(), 1);
        assert_eq!(viols[0].rule, MILLER_LIMIT);
    }

    #[test]
    fn deserialises_max_children_from_json_value() {
        let v: MillerLimitOptions = serde_json::from_value(serde_json::json!({
            "max_children": 4u32
        }))
        .unwrap();
        assert_eq!(v.max_children, 4);
    }
}
