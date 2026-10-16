use std::collections::HashMap;
use std::path::Path;

use camino::Utf8Path;
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};

use stratum_core::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId},
    types::ContainerKind,
};

use crate::builder::strip_windows_extended_prefix;
use crate::graph::CompoundGraph;

/// The graph facts of a snapshot; paths are relative to the project root and `/`-separated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphSnapshot {
    pub modules: Vec<SnapshotModule>,
    pub containers: Vec<SnapshotContainer>,
    pub layers: Vec<SnapshotLayer>,
    pub edges: Vec<SnapshotEdge>,
    pub zones: Vec<SnapshotZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotModule {
    pub id: ModuleId,
    pub path: String,
    pub container: ContainerId,
    pub layer: LayerId,
    pub purity: u8,
    /// Index into [`GraphSnapshot::zones`] of the innermost private zone holding the module.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotContainer {
    pub id: ContainerId,
    pub name: String,
    pub path: String,
    pub kind: ContainerKind,
    pub layer: LayerId,
    pub parent: Option<ContainerId>,
    pub stage: u8,
    pub entry: Option<ModuleId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotLayer {
    pub id: LayerId,
    pub name: String,
    pub depends_on: Vec<LayerId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotEdge {
    pub from: ModuleId,
    pub to: ModuleId,
    pub kind: EdgeKind,
}

/// A [`CompoundGraph::private_zones`] entry; `id` is its index there.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotZone {
    pub id: u32,
    pub path: String,
    pub root: String,
}

/// Materialise a deterministic snapshot of `g`, its paths made relative to `root`.
#[must_use]
pub fn snapshot_of(g: &CompoundGraph, root: &Utf8Path) -> GraphSnapshot {
    let root = strip_windows_extended_prefix(root);
    let rel = |path: &Path| relative(path, root.as_std_path());
    let zones: Vec<SnapshotZone> = g
        .private_zones()
        .iter()
        .zip(0..)
        .map(|(z, id)| SnapshotZone {
            id,
            path: rel(&z.path),
            root: rel(&z.root),
        })
        .collect();
    let zone_of: HashMap<ModuleId, u32> = g
        .private_zones()
        .iter()
        .zip(0..)
        .flat_map(|(z, id)| z.files.iter().map(move |&m| (m, id)))
        .collect();

    let mut modules: Vec<SnapshotModule> = g
        .modules
        .values()
        .map(|m| SnapshotModule {
            id: m.id,
            path: rel(&m.path),
            container: m.container,
            layer: m.layer,
            purity: m.purity.rank(),
            zone: zone_of.get(&m.id).copied(),
        })
        .collect();
    modules.sort_by_key(|m| m.id.raw());

    let mut containers: Vec<SnapshotContainer> = g
        .containers
        .values()
        .map(|c| SnapshotContainer {
            id: c.id,
            name: c.name.clone(),
            path: rel(&c.path),
            kind: c.kind,
            layer: c.layer,
            parent: c.parent,
            stage: c.stage.rank(),
            entry: c.entry,
        })
        .collect();
    containers.sort_by_key(|c| c.id.raw());

    let mut layers: Vec<SnapshotLayer> = g
        .layers
        .values()
        .map(|l| SnapshotLayer {
            id: l.id,
            name: l.name.clone(),
            depends_on: l.depends_on.clone(),
        })
        .collect();
    layers.sort_by_key(|l| l.id.raw());

    let mut edges: Vec<SnapshotEdge> = g
        .deps
        .edge_references()
        .map(|e| SnapshotEdge {
            from: g.deps[e.source()],
            to: g.deps[e.target()],
            kind: *e.weight(),
        })
        .collect();
    edges.sort_by(|a, b| {
        (a.from.raw(), a.to.raw(), a.kind as u8).cmp(&(b.from.raw(), b.to.raw(), b.kind as u8))
    });

    GraphSnapshot {
        modules,
        containers,
        layers,
        edges,
        zones,
    }
}

fn relative(path: &Path, root: &Path) -> String {
    let rest = path.strip_prefix(root).unwrap_or(path);
    rest.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_snapshot() {
        let g = CompoundGraph::empty();
        let s = snapshot_of(&g, Utf8Path::new(""));
        assert!(s.modules.is_empty());
        assert!(s.edges.is_empty());
    }
}
