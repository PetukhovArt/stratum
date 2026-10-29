//! Compound DAG construction and graph algorithms.

#![forbid(unsafe_code)]

pub mod builder;
pub mod contract;
pub mod cycles;
pub mod graph;
mod index;
pub mod metrics;
pub mod ownership;
mod placement;
pub mod snapshot;
pub mod topo;

pub use builder::{BuildConfig, BuildError, CacheState, CacheStats, GraphBuilder};
pub use contract::{CrossingName, Declaration, crossing_names};
pub use cycles::{
    Breaks, Cut, Cycle, break_candidates, break_candidates_where, find_cycles, find_cycles_where,
};
pub use graph::{CompoundGraph, Declared, Manifest, NavDoc, PrivateZone, SideFile, SideRole};
pub use index::{DirEntries, DirRole};
pub use snapshot::{
    GraphSnapshot, SnapshotContainer, SnapshotEdge, SnapshotLayer, SnapshotModule, SnapshotZone,
    snapshot_of,
};
pub use stratum_parser_ts::{Overlay, TrackedCalls};
pub use topo::{direct_dependencies, direct_dependents};
