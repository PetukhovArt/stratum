//! Compound DAG construction and graph algorithms.

#![forbid(unsafe_code)]

pub mod graph;
pub mod layer_assignment;

pub use graph::CompoundGraph;
pub use layer_assignment::assign_layer;
