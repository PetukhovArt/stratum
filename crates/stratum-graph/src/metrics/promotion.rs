use stratum_core::ids::ModuleId;

use crate::graph::CompoundGraph;

/// Distinct importers of `m` from other containers whose layer does not list
/// `m`'s layer in `depends_on`: the imports a move of `m` to a lower layer would
/// make legal. The declared rest is [`super::miller_fan_in`].
#[must_use]
pub fn promotion_pressure(g: &CompoundGraph, m: ModuleId) -> u32 {
    u32::try_from(super::cross_container_importers(g, m, false)).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::graph::CompoundGraph;
    use crate::metrics::miller_fan_in;
    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
    use std::path::PathBuf;
    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::{Layer, Module},
        visibility::VisibilityScope,
    };

    fn m(id: u32, layer: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(layer),
            layer: LayerId::new(layer),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: stratum_core::types::Exports::default(),
        }
    }

    fn layer(id: u32, depends_on: &[u32]) -> Layer {
        Layer {
            id: LayerId::new(id),
            name: format!("l{id}"),
            path: PathBuf::from(format!("src/l{id}")),
            depends_on: depends_on.iter().copied().map(LayerId::new).collect(),
        }
    }

    fn build(layers: Vec<Layer>, mods: Vec<Module>, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: layers.into_iter().map(|l| (l.id, l)).collect(),
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
    fn declared_and_undeclared_importers_split_between_miller_and_promotion() {
        let g = build(
            vec![layer(1, &[]), layer(2, &[1]), layer(3, &[])],
            vec![m(0, 1), m(1, 1), m(2, 2), m(3, 2), m(4, 3)],
            &[(1, 0), (2, 0), (3, 0), (4, 0)],
        );
        assert_eq!(miller_fan_in(&g, ModuleId::new(0)), 2);
        assert_eq!(promotion_pressure(&g, ModuleId::new(0)), 1);
    }
}
