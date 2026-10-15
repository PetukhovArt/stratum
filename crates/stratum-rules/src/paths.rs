use std::path::{Component, Path, PathBuf};

use stratum_core::ids::LayerId;
use stratum_graph::CompoundGraph;

pub(crate) fn project_relative(graph: &CompoundGraph, layer: LayerId, path: &Path) -> String {
    let layer_path: Option<PathBuf> = graph.layers.get(&layer).map(|l| {
        l.path
            .components()
            .filter(|c| matches!(c, Component::Normal(_)))
            .collect()
    });
    let rel = layer_path.and_then(|lp| {
        let ancestors: Vec<&Path> = path.ancestors().collect();
        let shallowest_root = ancestors
            .into_iter()
            .rev()
            .find(|a| path.starts_with(a.join(&lp)))?;
        path.strip_prefix(shallowest_root).ok()
    });
    rel.unwrap_or(path).to_string_lossy().replace('\\', "/")
}
