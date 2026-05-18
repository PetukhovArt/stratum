use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};

use stratum_core::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId},
};

use crate::graph::CompoundGraph;

pub const CURRENT_SNAPSHOT_VERSION: u32 = 1;

/// On-disk / on-wire shape of the compound graph.
/// **Versioned.** Bumps require a coordinated frontend release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphSnapshot {
    pub version: u32,
    pub modules: Vec<SnapshotModule>,
    pub containers: Vec<SnapshotContainer>,
    pub layers: Vec<SnapshotLayer>,
    pub edges: Vec<SnapshotEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotModule {
    pub id: ModuleId,
    pub path: String,
    pub container: ContainerId,
    pub layer: LayerId,
    pub stage: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotContainer {
    pub id: ContainerId,
    pub name: String,
    pub layer: LayerId,
    pub parent: Option<ContainerId>,
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

/// Materialise a deterministic snapshot of `g`.
#[must_use]
pub fn snapshot_of(g: &CompoundGraph) -> GraphSnapshot {
    let mut modules: Vec<SnapshotModule> = g
        .modules
        .values()
        .map(|m| SnapshotModule {
            id: m.id,
            path: m.path.to_string_lossy().to_string(),
            container: m.container,
            layer: m.layer,
            stage: m.stage.rank(),
        })
        .collect();
    modules.sort_by_key(|m| m.id.raw());

    let mut containers: Vec<SnapshotContainer> = g
        .containers
        .values()
        .map(|c| SnapshotContainer {
            id: c.id,
            name: c.name.clone(),
            layer: c.layer,
            parent: c.parent,
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
        version: CURRENT_SNAPSHOT_VERSION,
        modules,
        containers,
        layers,
        edges,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_snapshot() {
        let g = CompoundGraph::empty();
        let s = snapshot_of(&g);
        assert_eq!(s.version, 1);
        assert!(s.modules.is_empty());
        assert!(s.edges.is_empty());
    }

    #[test]
    fn snapshot_roundtrips_json() {
        let s = GraphSnapshot {
            version: 1,
            modules: vec![],
            containers: vec![],
            layers: vec![],
            edges: vec![],
        };
        let j = serde_json::to_string(&s).unwrap();
        let back: GraphSnapshot = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);
    }
}
