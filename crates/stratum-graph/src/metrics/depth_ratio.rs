use stratum_core::ids::{ContainerId, ModuleId};

use crate::graph::CompoundGraph;

/// Depth ratio = container_internal_count / public_exports.
///
/// Rewards encapsulation: a deep module exposes little of what it does.
/// `public_exports` is supplied by the caller — Phase 1 does not yet count
/// export sites; Phase 5 will pipe real counts in.
///
/// Returns `None` if the module is unknown or `public_exports == 0`.
#[must_use]
pub fn depth_ratio(g: &CompoundGraph, module: ModuleId, public_exports: u32) -> Option<f64> {
    let m = g.modules.get(&module)?;
    let container = m.container;
    let internal = container_internal_count(g, container);
    if public_exports == 0 {
        return None;
    }
    Some(internal as f64 / f64::from(public_exports))
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
        stage::Stage,
        types::Module,
        visibility::VisibilityScope,
    };

    fn m(container: u32, id: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(container),
            layer: LayerId::new(0),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
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
        let g = small(vec![m(1, 0), m(1, 1), m(1, 2), m(1, 3)]);
        assert_eq!(depth_ratio(&g, ModuleId::new(0), 2), Some(2.0));
    }

    #[test]
    fn zero_exports_returns_none() {
        let g = small(vec![m(1, 0)]);
        assert_eq!(depth_ratio(&g, ModuleId::new(0), 0), None);
    }
}
