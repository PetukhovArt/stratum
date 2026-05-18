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
use stratum_graph::{CompoundGraph, find_cycles, snapshot_of};

fn arb_graph(n: usize) -> impl Strategy<Value = (usize, Vec<(u32, u32)>)> {
    let mut candidate_edges = Vec::new();
    for a in 0u32..n as u32 {
        for b in 0u32..n as u32 {
            if a != b {
                candidate_edges.push((a, b));
            }
        }
    }
    let edge_count = candidate_edges.len();
    proptest::collection::vec(any::<bool>(), edge_count).prop_map(move |bits| {
        let picked: Vec<(u32, u32)> = candidate_edges
            .iter()
            .zip(bits.iter())
            .filter_map(|(e, &b)| if b { Some(*e) } else { None })
            .collect();
        (n, picked)
    })
}

#[allow(clippy::unwrap_used)]
fn build(n: usize, edges: &[(u32, u32)]) -> CompoundGraph {
    let mut g = CompoundGraph {
        modules: IndexMap::new(),
        containers: IndexMap::new(),
        layers: IndexMap::new(),
        deps: DiGraph::new(),
        node_index: FxHashMap::default(),
    };
    for i in 0..n {
        let id = ModuleId::new(i as u32);
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
    fn snapshot_is_deterministic((n, edges) in arb_graph(8)) {
        let g = build(n, &edges);
        let s1 = snapshot_of(&g);
        let s2 = snapshot_of(&g);
        prop_assert_eq!(s1, s2);
    }

    #[test]
    fn snapshot_preserves_module_and_edge_counts((n, edges) in arb_graph(8)) {
        let g = build(n, &edges);
        let s = snapshot_of(&g);
        prop_assert_eq!(s.modules.len(), n);
        prop_assert_eq!(s.edges.len(), edges.len());
    }

    #[test]
    fn cycles_never_panics((n, edges) in arb_graph(6)) {
        let g = build(n, &edges);
        let cycles = find_cycles(&g);
        for c in &cycles {
            prop_assert!(!c.modules.is_empty());
        }
    }
}
