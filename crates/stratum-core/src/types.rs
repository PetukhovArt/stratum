use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    ids::{ContainerId, LayerId, ModuleId},
    purity::Purity,
    stage::Stage,
    violation::SourceLocation,
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
    #[serde(default)]
    pub calls: Vec<Call>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    pub name: String,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exports {
    /// Distinct exported names, plus 1 for every `export * from`.
    pub count: u32,
    /// How many `export * from` entries `count` holds; each hides an unknown number of names.
    pub star_reexports: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerKind {
    Layer,
    Slice,
    SubUnit,
    Nested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    pub id: ContainerId,
    pub name: String,
    pub path: PathBuf,
    pub kind: ContainerKind,
    pub layer: LayerId,
    pub parent: Option<ContainerId>,
    pub stage: Stage,
    pub entry: Option<ModuleId>,
}

/// A named group of Modules with its own visibility and dependency direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub path: PathBuf,
    pub depends_on: Vec<LayerId>,
    pub slices: bool,
}
