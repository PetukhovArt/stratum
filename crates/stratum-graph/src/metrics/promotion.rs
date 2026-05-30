use stratum_core::ids::ModuleId;

use crate::graph::CompoundGraph;
use crate::topo::direct_dependents;

/// "Promotion pressure" on a module `m`: the count of distinct cross-container
/// dependents of `m`. High pressure suggests `m` should be promoted to a lower
/// layer to make its dependencies legal.
///
/// Phase 5 implementation: the metric returns the count of dependents whose
/// container differs from `m`'s. Phase 6+ may refine to weight by layer
/// distance.
#[must_use]
pub fn promotion_pressure(g: &CompoundGraph, m: ModuleId) -> u32 {
    let Some(target) = g.modules.get(&m) else {
        return 0;
    };
    let own_container = target.container;
    let count = direct_dependents(g, m)
        .into_iter()
        .filter(|dep| {
            g.modules
                .get(dep)
                .is_some_and(|md| md.container != own_container)
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::graph::CompoundGraph;
    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
    use std::path::PathBuf;
    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::Module,
        visibility::VisibilityScope,
    };

    fn m(id: u32, container: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
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
        for mm in mods {
            let node = g.deps.add_node(mm.id);
            g.node_index.insert(mm.id, node);
            g.modules.insert(mm.id, mm);
        }
        for (a, b) in edges {
            let na = g.node_index[&ModuleId::new(*a)];
            let nb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn empty_when_no_dependents() {
        let g = build(vec![m(0, 1)], &[]);
        assert_eq!(promotion_pressure(&g, ModuleId::new(0)), 0);
    }

    #[test]
    fn high_when_many_cross_container_dependents() {
        let mut mods = vec![m(0, 1)];
        for i in 1..=6u32 {
            mods.push(m(i, 2));
        }
        let edges: Vec<(u32, u32)> = (1..=6).map(|i| (i, 0)).collect();
        let g = build(mods, &edges);
        assert_eq!(promotion_pressure(&g, ModuleId::new(0)), 6);
    }

    #[test]
    fn ignores_same_container_dependents() {
        let g = build(vec![m(0, 1), m(1, 1)], &[(1, 0)]);
        assert_eq!(promotion_pressure(&g, ModuleId::new(0)), 0);
    }
}
