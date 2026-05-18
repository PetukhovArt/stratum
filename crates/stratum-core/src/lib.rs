//! Domain types and Salsa database for Stratum tooling.

pub mod ids;
pub mod severity;
pub mod stage;
pub mod visibility;

pub use ids::{ContainerId, LayerId, ModuleId, ProjectId, RuleId};
pub use severity::Severity;
pub use stage::{InvalidStage, Stage};
pub use visibility::VisibilityScope;
