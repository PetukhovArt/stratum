#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod baseline;
pub mod cache;
pub mod co_change;
pub mod config;
mod docs;
pub mod drift;
mod features;
pub mod finding;
mod manifest;
pub mod paths;
pub mod pipeline;
mod side;
pub mod snapshot;
pub mod suppression;
mod systems;
mod zero_config;

pub use baseline::{BASELINE_FILE, Baseline, BaselineError, Growth, Matched};
pub use config::{CONFIG_FILE, ConfigSource, LoadedConfig};
pub use drift::{CycleChange, Drift, FanInGrowth, LayerEdgeGrowth, SliceEdge, StageChange, drift};
pub use features::{Feature, Features};
pub use finding::{EdgeRef, Finding};
pub use pipeline::{EngineError, EngineInput};
pub use snapshot::{SNAPSHOT_VERSION, Snapshot};
pub use suppression::{Analysis, Suppressed};
pub use systems::{CalledContract, Caller, Channel, Decl, System, SystemMap};

pub const SOURCE_EXTENSIONS: [&str; 9] =
    ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "vue"];

#[must_use]
pub fn is_source(path: &camino::Utf8Path) -> bool {
    path.extension()
        .is_some_and(|e| SOURCE_EXTENSIONS.contains(&e))
}
