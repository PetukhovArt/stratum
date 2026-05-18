#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use indexmap::IndexMap;
use petgraph::graph::DiGraph;
use proptest::prelude::*;
use rustc_hash::FxHashMap;

use stratum_core::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId},
    severity::Severity,
    stage::Stage,
    types::Module,
    visibility::VisibilityScope,
};
use stratum_graph::CompoundGraph;
use stratum_rules::{EmptyOptions, NoCircularDeps, ProjectScope, Rule};

fn arb_edges(n: u32) -> impl Strategy<Value = Vec<(u32, u32)>> {
    let mut all = Vec::new();
    for a in 0..n {
        for b in 0..n {
            if a != b {
                all.push((a, b));
            }
        }
    }
    let count = all.len();
    proptest::collection::vec(any::<bool>(), count).prop_map(move |bits| {
        all.iter()
            .zip(bits)
            .filter_map(|(e, b)| if b { Some(*e) } else { None })
            .collect()
    })
}

fn build(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
    let mut g = CompoundGraph {
        modules: IndexMap::new(),
        containers: IndexMap::new(),
        layers: IndexMap::new(),
        deps: DiGraph::new(),
        node_index: FxHashMap::default(),
    };
    for i in 0..n {
        let id = ModuleId::new(i);
        let m = Module {
            id,
            path: PathBuf::from(format!("m{i}.ts")),
            container: ContainerId::new(0),
            layer: LayerId::new(0),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
        };
        let node = g.deps.add_node(id);
        g.node_index.insert(id, node);
        g.modules.insert(id, m);
    }
    for (a, b) in edges {
        let na = g.node_index[&ModuleId::new(*a)];
        let nb = g.node_index[&ModuleId::new(*b)];
        g.deps.add_edge(na, nb, EdgeKind::Static);
    }
    g
}

proptest! {
    #[test]
    fn no_circular_deps_deterministic(edges in arb_edges(6)) {
        let g = build(6, &edges);
        let r = NoCircularDeps;
        let a = r.check(&g, ProjectScope, &EmptyOptions, Severity::Error);
        let b = r.check(&g, ProjectScope, &EmptyOptions, Severity::Error);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn no_circular_deps_never_panics(edges in arb_edges(6)) {
        let g = build(6, &edges);
        let r = NoCircularDeps;
        let _ = r.check(&g, ProjectScope, &EmptyOptions, Severity::Error);
    }
}
