//! Domain types for Stratum tooling.
//!
//! Stratum primitives ([`Layer`], [`Purity`], [`VisibilityScope`],
//! [`Container`], [`EdgeKind`]) are first-class types, not configuration
//! over generic primitives (PRD decision D1).

#![forbid(unsafe_code)]

pub mod edge;
pub mod ids;
pub mod purity;
pub mod severity;
pub mod stage;
pub mod structure;
pub mod types;
pub mod violation;
pub mod visibility;

pub use crate::{
    edge::{Edge, EdgeKind},
    ids::{ContainerId, LayerId, ModuleId, RuleId},
    purity::{InvalidPurity, Purity},
    severity::Severity,
    stage::{InvalidStage, Stage},
    structure::Structure,
    types::{
        Bindings, Call, Container, ContainerKind, ExportDecl, ExportOrigin, Exports, Import,
        ImportedName, Layer, Module, Suppression,
    },
    violation::{SourceLocation, Violation},
    visibility::VisibilityScope,
};
