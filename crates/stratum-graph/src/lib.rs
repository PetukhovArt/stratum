//! Compound DAG construction and graph algorithms.

#![forbid(unsafe_code)]

pub mod builder;
pub mod cycles;
pub mod graph;
mod index;
pub mod metrics;
pub mod ownership;
mod placement;
pub mod snapshot;
pub mod topo;

pub use builder::{BuildConfig, BuildError, GraphBuilder};
pub use cycles::{Breaks, Cut, Cycle, break_candidates, find_cycles};
pub use graph::{CompoundGraph, NavDoc, PrivateZone};
pub use index::{DirEntries, DirRole};
pub use snapshot::{
    GraphSnapshot, SnapshotContainer, SnapshotEdge, SnapshotLayer, SnapshotModule, SnapshotZone,
    snapshot_of,
};
pub use stratum_parser_ts::{Overlay, TrackedCalls};
pub use topo::{direct_dependencies, direct_dependents};
