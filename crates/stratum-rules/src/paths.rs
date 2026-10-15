use std::path::{Component, Path};

use stratum_core::ids::LayerId;
use stratum_graph::CompoundGraph;

pub(crate) fn project_relative(graph: &CompoundGraph, layer: LayerId, path: &Path) -> String {
    let rel = graph
        .layers
        .get(&layer)
        .and_then(|l| from_shallowest_layer_root(path, &l.path));
    rel.unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn from_shallowest_layer_root<'a>(path: &'a Path, layer_path: &Path) -> Option<&'a Path> {
    let mut rest = path.components();
    while matches!(
        rest.clone().next(),
        Some(Component::Prefix(_) | Component::RootDir)
    ) {
        rest.next();
    }
    loop {
        let mut probe = rest.clone();
        let starts_with_layer = layer_path
            .components()
            .filter(|c| matches!(c, Component::Normal(_)))
            .all(|c| probe.next() == Some(c));
        if starts_with_layer {
            return Some(rest.as_path());
        }
        rest.next()?;
    }
}
