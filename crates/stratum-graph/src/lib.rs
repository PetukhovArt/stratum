//! Compound DAG construction and graph algorithms.

#![forbid(unsafe_code)]

pub mod builder;
pub mod graph;
pub mod layer_assignment;

pub use builder::{BuildConfig, BuildError, GraphBuilder};
pub use graph::CompoundGraph;
pub use layer_assignment::assign_layer;
