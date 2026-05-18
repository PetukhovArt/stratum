use stratum_core::ids::{ContainerId, ModuleId};

use crate::graph::CompoundGraph;
use crate::topo::direct_dependents;

/// Number of distinct dependents of `module` outside its own container.
///
/// The Miller fan-out feeds the `miller-limit` rule (Phase 3+): a high count
/// indicates wide cross-container coupling.
#[must_use]
pub fn miller_fanout(g: &CompoundGraph, module: ModuleId) -> usize {
    let Some(target) = g.modules.get(&module) else {
        return 0;
    };
    let own_container: ContainerId = target.container;
    direct_dependents(g, module)
        .into_iter()
        .filter(|dep| {
            g.modules
                .get(dep)
                .is_some_and(|m| m.container != own_container)
        })
        .count()
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
        ids::LayerId,
        stage::Stage,
        types::Module,
        visibility::VisibilityScope,
    };

    fn module_in(container_raw: u32, id: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(container_raw),
            layer: LayerId::new(0),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
        }
    }

    fn small_graph(modules: Vec<Module>, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
        for m in modules {
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
    fn fanout_counts_only_cross_container_dependents() {
        let g = small_graph(
            vec![
                module_in(100, 0),
                module_in(100, 1),
                module_in(200, 2),
                module_in(200, 3),
            ],
            &[(1, 0), (2, 0), (3, 0)],
        );
        assert_eq!(miller_fanout(&g, ModuleId::new(0)), 2);
    }
}
