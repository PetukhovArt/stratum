use stratum_core::ids::{ContainerId, ModuleId};

use crate::graph::CompoundGraph;

/// Depth ratio = modules in `module`'s container / `module`'s export count.
///
/// Returns `None` if the module is unknown or exports nothing.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn depth_ratio(g: &CompoundGraph, module: ModuleId) -> Option<f64> {
    let m = g.modules.get(&module)?;
    if m.exports.count == 0 {
        return None;
    }
    let internal = container_internal_count(g, m.container);
    Some(internal as f64 / f64::from(m.exports.count))
}

fn container_internal_count(g: &CompoundGraph, c: ContainerId) -> usize {
    g.modules.values().filter(|m| m.container == c).count()
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
    fn zero_exports_returns_none() {
        let g = small(vec![m(1, 0, 0)]);
        assert_eq!(depth_ratio(&g, ModuleId::new(0)), None);
    }
}
