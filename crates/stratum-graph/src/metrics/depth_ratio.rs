use stratum_core::ids::{ContainerId, ModuleId};

use crate::graph::CompoundGraph;

/// Depth ratio = modules in the subtree of `module`'s outer container / `module`'s export count.
///
/// Returns `None` if the module is unknown or exports nothing.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn depth_ratio(g: &CompoundGraph, module: ModuleId) -> Option<f64> {
    let m = g.modules.get(&module)?;
    if m.exports.count == 0 {
        return None;
    }
    let size = subtree_module_count(g, super::outer_container(g, m));
    Some(size as f64 / f64::from(m.exports.count))
}

/// Modules held by `c` or by any container nested in it.
#[must_use]
pub fn subtree_module_count(g: &CompoundGraph, c: ContainerId) -> usize {
    g.modules
        .values()
        .filter(|m| m.container == c || g.containers_of(m.id).any(|x| x.id == c))
        .count()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
    use std::path::PathBuf;
    use stratum_core::{
        ids::LayerId,
        purity::Purity,
        types::{Exports, Module},
        visibility::VisibilityScope,
    };

    fn m(container: u32, id: u32, exports: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(container),
            layer: LayerId::new(0),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: Exports {
                count: exports,
                star_reexports: 0,
            },
            calls: vec![],
        }
    }

    fn small(modules: Vec<Module>) -> CompoundGraph {
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
        g
    }

    #[test]
    fn depth_ratio_is_internal_over_exports() {
        let g = small(vec![m(1, 0, 2), m(1, 1, 0), m(1, 2, 0), m(1, 3, 0)]);
        assert_eq!(depth_ratio(&g, ModuleId::new(0)), Some(2.0));
    }

    #[test]
    fn size_is_the_outer_slice_subtree() {
        use stratum_core::{
            stage::Stage,
            types::{Container, ContainerKind},
        };
        let mut g = small(vec![m(2, 0, 1), m(3, 1, 0), m(4, 2, 0), m(0, 3, 0)]);
        for (id, kind, parent) in [
            (0, ContainerKind::Layer, None),
            (1, ContainerKind::Slice, Some(0)),
            (2, ContainerKind::SubUnit, Some(1)),
            (3, ContainerKind::Nested, Some(2)),
            (4, ContainerKind::Slice, Some(0)),
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
                },
            );
        }
        assert_eq!(depth_ratio(&g, ModuleId::new(0)), Some(2.0));
    }

    #[test]
    fn zero_exports_returns_none() {
        let g = small(vec![m(1, 0, 0)]);
        assert_eq!(depth_ratio(&g, ModuleId::new(0)), None);
    }
}
