//! Graph metrics. Values only — rules that interpret them live in `stratum-rules`.

pub mod depth_ratio;
pub mod miller;
pub mod promotion;

pub use depth_ratio::depth_ratio;
pub use miller::miller_fan_in;
pub use promotion::promotion_pressure;

use stratum_core::ids::{ContainerId, ModuleId};
use stratum_core::types::Module;

use crate::graph::CompoundGraph;
use crate::topo::direct_dependents;

fn outer_container(g: &CompoundGraph, m: &Module) -> ContainerId {
    g.outer_container(m.id).map_or(m.container, |c| c.id)
}

fn cross_container_importers(g: &CompoundGraph, module: ModuleId, want_declared: bool) -> usize {
    let Some(target) = g.modules.get(&module) else {
        return 0;
    };
    let target_container = outer_container(g, target);
    direct_dependents(g, module)
        .into_iter()
        .filter_map(|dep| g.modules.get(&dep))
        .filter(|importer| outer_container(g, importer) != target_container)
        .filter(|importer| {
            let declares = importer.layer == target.layer
                || g.layers
                    .get(&importer.layer)
                    .is_some_and(|l| l.depends_on.contains(&target.layer));
            declares == want_declared
        })
        .count()
}
