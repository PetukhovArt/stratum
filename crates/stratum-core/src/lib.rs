//! Domain types and Salsa database for Stratum tooling.

pub mod ids;
pub mod severity;

pub use ids::{ContainerId, LayerId, ModuleId, ProjectId, RuleId};
pub use severity::Severity;
