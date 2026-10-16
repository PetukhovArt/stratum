use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use stratum_config::ConfigError;
use stratum_core::{
    ids::{ContainerId, LayerId},
    types::ContainerKind,
};
use stratum_graph::{
    SnapshotContainer, SnapshotEdge, SnapshotLayer, SnapshotModule, SnapshotZone, snapshot_of,
};
use stratum_rules::territory::{self, Site, Territories};

use crate::pipeline::EngineInput;

pub const SNAPSHOT_VERSION: u32 = 3;

/// The Graph Snapshot: the on-disk / on-wire contract with the Visualizer.
/// **Versioned.** Bumps require a coordinated frontend release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snapshot {
    pub version: u32,
    /// The project root's directory name.
    pub project: String,
    pub modules: Vec<Module>,
    pub containers: Vec<SnapshotContainer>,
    pub layers: Vec<SnapshotLayer>,
    pub edges: Vec<SnapshotEdge>,
    pub zones: Vec<SnapshotZone>,
    pub territories: Vec<Territory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Module {
    #[serde(flatten)]
    pub graph: SnapshotModule,
    /// Index into [`Snapshot::territories`].
    pub territory: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Territory {
    pub may_import_layers: Vec<LayerId>,
    pub imported_by_layers: Vec<LayerId>,
    pub max_import_purity: u8,
    /// Indices into [`Snapshot::zones`].
    pub visible_zones: Vec<u32>,
    pub may_not: Vec<Forbidden>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Forbidden {
    /// Rule slug.
    pub rule: String,
    pub region: Region,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Region {
    Siblings {
        parent: ContainerId,
        #[serde(rename = "containerKind")]
        container_kind: ContainerKind,
        containers: Vec<ContainerId>,
    },
    Internals,
}

impl EngineInput {
    pub fn snapshot(&self) -> Result<Snapshot, ConfigError> {
        let graph = snapshot_of(&self.graph, &self.project_root);
        let territories = Territories::new(
            &self.graph,
            &self.config,
            &self.project_root,
            &self.registry,
        )?;
        let mut index: HashMap<territory::Territory, u32> = HashMap::new();
        let mut distinct = Vec::new();
        let modules = graph
            .modules
            .into_iter()
            .map(|m| {
                let t = territories.of(Site::Module(m.id));
                let next = u32::try_from(distinct.len()).unwrap_or(u32::MAX);
                let territory = *index.entry(t).or_insert_with_key(|t| {
                    distinct.push(self.wire_territory(t));
                    next
                });
                Module {
                    graph: m,
                    territory,
                }
            })
            .collect();
        Ok(Snapshot {
            version: SNAPSHOT_VERSION,
            project: self.project_root.file_name().unwrap_or_default().to_owned(),
            modules,
            containers: graph.containers,
            layers: graph.layers,
            edges: graph.edges,
            zones: graph.zones,
            territories: distinct,
        })
    }

    fn wire_territory(&self, t: &territory::Territory) -> Territory {
        Territory {
            may_import_layers: t.may_import_layers.clone(),
            imported_by_layers: t.imported_by_layers.clone(),
            max_import_purity: t.max_import_purity,
            visible_zones: t
                .visible_zones
                .iter()
                .map(|z| u32::try_from(z.index()).unwrap_or(u32::MAX))
                .collect(),
            may_not: t
                .may_not
                .iter()
                .map(|f| Forbidden {
                    rule: self.slug_of(f.rule).to_owned(),
                    region: match &f.region {
                        territory::Region::Siblings {
                            parent,
                            kind,
                            containers,
                        } => Region::Siblings {
                            parent: *parent,
                            container_kind: *kind,
                            containers: containers.clone(),
                        },
                        territory::Region::Internals => Region::Internals,
                    },
                })
                .collect(),
        }
    }
}
