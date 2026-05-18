//! # stratum-core
//!
//! Domain types and the incremental [`ArchitectureDatabase`] for Stratum
//! tooling.
//!
//! ## Public surface
//!
//! `stratum-core` deliberately exposes **only three deep queries** on the
//! [`ArchitectureDatabase`] trait — `compound_graph`, `violations`, and
//! `violations_for_file`. Intermediate Salsa queries (source parsing,
//! module info, import extraction, rule execution) are crate-private.
//! See PRD decision D6 for the reasoning.
//!
//! ## Stratum primitives are first-class
//!
//! [`Layer`], [`Stage`], [`VisibilityScope`], [`Container`], and the
//! distinction between `static`/`di`/`runtime` [`EdgeKind`]s are types
//! in this crate — not configuration on top of generic primitives. The
//! tool is opinionated about the methodology (PRD decision D1).

#![forbid(unsafe_code)]

pub mod db;
pub mod edge;
pub mod ids;
pub mod severity;
pub mod stage;
pub mod types;
pub mod violation;
pub mod visibility;

pub use crate::{
    db::{ArchitectureDatabase, CompoundGraphSnapshot, Project, StratumDb},
    edge::{Edge, EdgeKind},
    ids::{ContainerId, LayerId, ModuleId, ProjectId, RuleId},
    severity::Severity,
    stage::{InvalidStage, Stage},
    types::{Container, Layer, Module},
    violation::{SourceLocation, Violation},
    visibility::VisibilityScope,
};

/// Convenience re-exports. Importing `stratum_core::prelude::*` brings
/// every public type into scope.
pub mod prelude {
    pub use crate::{
        ArchitectureDatabase, CompoundGraphSnapshot, Container, ContainerId, Edge, EdgeKind,
        InvalidStage, Layer, LayerId, Module, ModuleId, Project, ProjectId, RuleId, Severity,
        SourceLocation, Stage, StratumDb, Violation, VisibilityScope,
    };
}
