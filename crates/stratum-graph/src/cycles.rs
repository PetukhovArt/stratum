use petgraph::algo::tarjan_scc;

use stratum_core::ids::ModuleId;

use crate::graph::CompoundGraph;

/// A non-trivial strongly-connected component: a cycle of two or more modules,
/// or a singleton with an explicit self-loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    pub modules: Vec<ModuleId>,
}

/// Find every cycle: SCCs of size ≥ 2, plus singletons with a self-loop.
///
/// Modules within each cycle are sorted by their raw id; the returned vec is
/// also sorted by the first module's raw id — deterministic across runs.
#[must_use]
pub fn find_cycles(g: &CompoundGraph) -> Vec<Cycle> {
    let sccs = tarjan_scc(&g.deps);
    let mut cycles = Vec::new();
    for scc in sccs {
        if scc.len() >= 2 {
            let mut modules: Vec<ModuleId> = scc.iter().map(|n| g.deps[*n]).collect();
            modules.sort_by_key(|m| m.raw());
            cycles.push(Cycle { modules });
        } else if scc.len() == 1 {
            let n = scc[0];
            if g.deps.find_edge(n, n).is_some() {
                cycles.push(Cycle {
                    modules: vec![g.deps[n]],
                });
            }
        }
    }
    cycles.sort_by_key(|c| c.modules.first().map_or(u32::MAX, |m| m.raw()));
    cycles
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
    fn dag_has_no_cycles() {
        let g = graph_with_edges(3, &[(0, 1), (1, 2)]);
        assert!(find_cycles(&g).is_empty());
    }

    #[test]
    fn finds_two_node_cycle() {
        let g = graph_with_edges(2, &[(0, 1), (1, 0)]);
        let cycles = find_cycles(&g);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].modules, vec![ModuleId::new(0), ModuleId::new(1)]);
    }

    #[test]
    fn finds_three_node_cycle() {
        let g = graph_with_edges(3, &[(0, 1), (1, 2), (2, 0)]);
        let cycles = find_cycles(&g);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].modules.len(), 3);
    }

    #[test]
    fn finds_self_loop() {
        let g = graph_with_edges(1, &[(0, 0)]);
        let cycles = find_cycles(&g);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].modules, vec![ModuleId::new(0)]);
    }

    #[test]
    fn multiple_disjoint_cycles() {
        let g = graph_with_edges(4, &[(0, 1), (1, 0), (2, 3), (3, 2)]);
        let cycles = find_cycles(&g);
        assert_eq!(cycles.len(), 2);
        assert_eq!(cycles[0].modules[0], ModuleId::new(0));
        assert_eq!(cycles[1].modules[0], ModuleId::new(2));
    }
}
