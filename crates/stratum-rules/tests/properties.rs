#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use proptest::prelude::*;

use stratum_core::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId},
    purity::Purity,
    severity::Severity,
    types::Module,
    visibility::VisibilityScope,
};
use stratum_graph::CompoundGraph;
use stratum_rules::{NoCircularDeps, NoCircularDepsOptions, ProjectScope, Rule};

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
    let mut g = CompoundGraph::empty();
    for i in 0..n {
        let id = ModuleId::new(i);
        let m = Module {
            id,
            path: PathBuf::from(format!("m{i}.ts")),
            container: ContainerId::new(0),
            layer: LayerId::new(0),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: stratum_core::types::Exports::default(),
            calls: vec![],
            imports: vec![],
            suppressions: vec![],
            local_refs: vec![],
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
        let a = r.check(&g, ProjectScope, &NoCircularDepsOptions::default(), Severity::Error);
        let b = r.check(&g, ProjectScope, &NoCircularDepsOptions::default(), Severity::Error);
        prop_assert_eq!(a, b);
    }
}
