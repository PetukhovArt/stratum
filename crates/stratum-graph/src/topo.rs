use petgraph::Direction;
use petgraph::visit::EdgeRef;

use stratum_core::ids::ModuleId;

use crate::graph::CompoundGraph;

/// All modules that directly import `m`.
#[must_use]
pub fn direct_dependents(g: &CompoundGraph, m: ModuleId) -> Vec<ModuleId> {
    let Some(node) = g.node_for(m) else {
        return vec![];
    };
    let mut out: Vec<ModuleId> = g
        .deps
        .edges_directed(node, Direction::Incoming)
        .map(|e| g.deps[e.source()])
        .collect();
    out.sort_by_key(|m| m.raw());
    out.dedup();
    out
}

/// All modules that `m` directly imports.
#[must_use]
pub fn direct_dependencies(g: &CompoundGraph, m: ModuleId) -> Vec<ModuleId> {
    let Some(node) = g.node_for(m) else {
        return vec![];
    };
    let mut out: Vec<ModuleId> = g
        .deps
        .edges_directed(node, Direction::Outgoing)
        .map(|e| g.deps[e.target()])
        .collect();
    out.sort_by_key(|m| m.raw());
    out.dedup();
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use stratum_core::edge::EdgeKind;

    fn graph_with_edges(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for i in 0..n {
            let id = ModuleId::new(i);
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
        }
        for (a, b) in edges {
            let fa = g.node_index[&ModuleId::new(*a)];
            let fb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(fa, fb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn dependents_and_dependencies() {
        let g = graph_with_edges(3, &[(0, 1), (2, 1)]);
        assert_eq!(
            direct_dependents(&g, ModuleId::new(1)),
            vec![ModuleId::new(0), ModuleId::new(2)]
        );
        assert_eq!(
            direct_dependencies(&g, ModuleId::new(1)),
            Vec::<ModuleId>::new()
        );
        assert_eq!(
            direct_dependencies(&g, ModuleId::new(0)),
            vec![ModuleId::new(1)]
        );
    }
}
