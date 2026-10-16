use stratum_core::ids::ModuleId;

use crate::graph::CompoundGraph;

/// Distinct importers of `module` from other containers whose layer is `module`'s
/// layer or lists it in `depends_on`. The undeclared rest is [`super::promotion_pressure`].
#[must_use]
pub fn miller_fan_in(g: &CompoundGraph, module: ModuleId) -> usize {
    super::cross_container_importers(g, module, true)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::graph::CompoundGraph;

    use std::path::PathBuf;
    use stratum_core::ids::ContainerId;
    use stratum_core::{
        edge::EdgeKind, ids::LayerId, purity::Purity, types::Module, visibility::VisibilityScope,
    };

    fn module_in(container_raw: u32, id: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(container_raw),
            layer: LayerId::new(0),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: stratum_core::types::Exports::default(),
            calls: vec![],
            imports: vec![],
            suppressions: vec![],
        }
    }

    fn small_graph(modules: Vec<Module>, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
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
    fn fan_in_counts_only_cross_container_dependents() {
        let g = small_graph(
            vec![
                module_in(100, 0),
                module_in(100, 1),
                module_in(200, 2),
                module_in(200, 3),
            ],
            &[(1, 0), (2, 0), (3, 0)],
        );
        assert_eq!(miller_fan_in(&g, ModuleId::new(0)), 2);
    }

    #[test]
    fn sliced_layer_counts_importers_from_other_slices_only() {
        use stratum_core::{
            stage::Stage,
            types::{Container, ContainerKind},
        };
        let mut g = small_graph(
            vec![
                module_in(3, 0),
                module_in(4, 1),
                module_in(2, 2),
                module_in(5, 3),
            ],
            &[(1, 0), (2, 0), (3, 0)],
        );
        for (id, kind, parent) in [
            (0, ContainerKind::Layer, None),
            (1, ContainerKind::Slice, Some(0)),
            (2, ContainerKind::Slice, Some(0)),
            (3, ContainerKind::SubUnit, Some(1)),
            (4, ContainerKind::SubUnit, Some(1)),
            (5, ContainerKind::Nested, Some(2)),
        ] {
            let id = ContainerId::new(id);
            g.containers.insert(
                id,
                Container {
                    id,
                    name: format!("c{}", id.raw()),
                    path: PathBuf::from(format!("c{}", id.raw())),
                    kind,
                    layer: LayerId::new(0),
                    parent: parent.map(ContainerId::new),
                    stage: Stage::FLAT,
                    entry: None,
                    segment_dirs: vec![],
                },
            );
        }
        assert_eq!(miller_fan_in(&g, ModuleId::new(0)), 2);
    }
}
