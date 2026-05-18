use petgraph::Direction;
use petgraph::algo::toposort;
use petgraph::visit::EdgeRef;

use stratum_core::ids::ModuleId;

use crate::graph::CompoundGraph;

/// Topologically sort the dependency DAG.
///
/// # Errors
/// Returns `Err(cycle_module)` if the graph contains a cycle; the returned
/// module is one participant of an unspecified cycle (use
/// [`crate::cycles::find_cycles`] for full enumeration).
pub fn topo_sort(g: &CompoundGraph) -> Result<Vec<ModuleId>, ModuleId> {
    match toposort(&g.deps, None) {
        Ok(order) => Ok(order.into_iter().map(|n| g.deps[n]).collect()),
        Err(cycle) => Err(g.deps[cycle.node_id()]),
    }
}

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
    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
    use stratum_core::edge::EdgeKind;

    fn graph_with_edges(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
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
    fn toposort_dag() {
        let g = graph_with_edges(3, &[(0, 1), (1, 2)]);
        let order = topo_sort(&g).unwrap();
        let pos: std::collections::HashMap<_, _> =
            order.iter().enumerate().map(|(i, m)| (*m, i)).collect();
        assert!(pos[&ModuleId::new(0)] < pos[&ModuleId::new(1)]);
        assert!(pos[&ModuleId::new(1)] < pos[&ModuleId::new(2)]);
    }

    #[test]
    fn toposort_cycle_errors() {
        let g = graph_with_edges(2, &[(0, 1), (1, 0)]);
        assert!(topo_sort(&g).is_err());
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
