use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    ids::{ContainerId, LayerId, ModuleId},
    purity::Purity,
    visibility::VisibilityScope,
};

/// One source unit (file or SFC) appearing as a node in the Compound DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub path: PathBuf,
    pub container: ContainerId,
    pub layer: LayerId,
    pub purity: Purity,
    pub visibility: VisibilityScope,
    #[serde(default)]
    pub exports: Exports,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exports {
    /// Distinct exported names, plus 1 for every `export * from`.
    pub count: u32,
    /// How many `export * from` entries `count` holds; each hides an unknown number of names.
    pub star_reexports: u32,
}

/// A compound node grouping Modules of one Layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    pub id: ContainerId,
    pub name: String,
    pub layer: LayerId,
    pub parent: Option<ContainerId>,
}

/// A named group of Modules with its own visibility and dependency direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub path: PathBuf,
    pub depends_on: Vec<LayerId>,
}
