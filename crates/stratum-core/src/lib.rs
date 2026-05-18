//! Domain types and Salsa database for Stratum tooling.

pub mod db;
pub mod edge;
pub mod ids;
pub mod severity;
pub mod stage;
pub mod types;
pub mod violation;
pub mod visibility;

pub use db::{ArchitectureDatabase, CompoundGraphSnapshot, Project, StratumDb};
pub use edge::{Edge, EdgeKind};
pub use ids::{ContainerId, LayerId, ModuleId, ProjectId, RuleId};
pub use severity::Severity;
pub use stage::{InvalidStage, Stage};
pub use types::{Container, Layer, Module};
pub use violation::{SourceLocation, Violation};
pub use visibility::VisibilityScope;
