#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use indexmap::IndexMap;
use petgraph::graph::DiGraph;
use proptest::prelude::*;
use rustc_hash::FxHashMap;

use stratum_core::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId},
    stage::Stage,
    types::Module,
    visibility::VisibilityScope,
};
use stratum_graph::{
    CompoundGraph,
    metrics::{miller_fanout, promotion_pressure},
    topo::direct_dependents,
};

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
        let container = if i % 2 == 0 { 100 } else { 200 };
        let m = Module {
            id: ModuleId::new(i),
            path: PathBuf::from(format!("m{i}.ts")),
            container: ContainerId::new(container),
            layer: LayerId::new(0),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
        };
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

proptest! {
    #[test]
    fn miller_fanout_le_direct_dependents(edges in arb_edges(6)) {
        let g = build(6, &edges);
        for i in 0..6 {
            let m = ModuleId::new(i);
            let dep_count = direct_dependents(&g, m).len();
            let fanout = miller_fanout(&g, m);
            prop_assert!(fanout <= dep_count);
        }
    }

    #[test]
    fn promotion_pressure_le_direct_dependents(edges in arb_edges(6)) {
        let g = build(6, &edges);
        for i in 0..6 {
            let m = ModuleId::new(i);
            let dep_count = u32::try_from(direct_dependents(&g, m).len()).unwrap_or(u32::MAX);
            let pressure = promotion_pressure(&g, m);
            prop_assert!(pressure <= dep_count);
        }
    }
}
