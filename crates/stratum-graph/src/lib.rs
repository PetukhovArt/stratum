//! Compound DAG construction and graph algorithms.

#![forbid(unsafe_code)]

pub mod builder;
pub mod cycles;
pub mod graph;
pub mod layer_assignment;
pub mod topo;

pub use builder::{BuildConfig, BuildError, GraphBuilder};
pub use cycles::{Cycle, find_cycles};
pub use graph::CompoundGraph;
pub use layer_assignment::assign_layer;
pub use topo::{direct_dependencies, direct_dependents, topo_sort};
