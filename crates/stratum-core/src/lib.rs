//! Domain types and the incremental [`ArchitectureDatabase`] for Stratum
//! tooling. The public surface is deliberately narrow: three queries on
//! [`ArchitectureDatabase`] (`compound_graph`, `violations`,
//! `violations_for_file`) plus the domain types they exchange. Intermediate
//! Salsa queries are `pub(crate)` (PRD decision D6).
//!
//! Stratum primitives ([`Layer`], [`Stage`], [`VisibilityScope`],
//! [`Container`], [`EdgeKind`]) are first-class types, not configuration
//! over generic primitives (PRD decision D1).

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
