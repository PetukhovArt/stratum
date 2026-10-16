#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod baseline;
pub mod config;
pub mod drift;
pub mod finding;
pub mod paths;
pub mod pipeline;
pub mod snapshot;
mod zero_config;

pub use baseline::{BASELINE_FILE, Baseline, BaselineError, Growth, Matched};
pub use config::{CONFIG_FILE, ConfigSource, LoadedConfig};
pub use drift::{CycleChange, Drift, FanInGrowth, LayerEdgeGrowth, SliceEdge, StageChange, drift};
pub use finding::{EdgeRef, Finding};
pub use pipeline::{EngineError, EngineInput};
pub use snapshot::{SNAPSHOT_VERSION, Snapshot};

pub const SOURCE_EXTENSIONS: [&str; 9] =
    ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "vue"];

#[must_use]
pub fn is_source(path: &camino::Utf8Path) -> bool {
    path.extension()
        .is_some_and(|e| SOURCE_EXTENSIONS.contains(&e))
}
