//! Compound DAG construction and graph algorithms.

#![forbid(unsafe_code)]

pub mod builder;
pub mod cycles;
pub mod graph;
mod index;
pub mod metrics;
mod placement;
pub mod snapshot;
pub mod topo;

pub use builder::{BuildConfig, BuildError, GraphBuilder};
pub use cycles::{Cycle, find_cycles};
pub use graph::{CompoundGraph, PrivateZone};
pub use index::DirEntries;
pub use snapshot::{
    CURRENT_SNAPSHOT_VERSION, GraphSnapshot, SnapshotContainer, SnapshotEdge, SnapshotLayer,
    SnapshotModule, snapshot_of,
};
pub use topo::{direct_dependencies, direct_dependents};
