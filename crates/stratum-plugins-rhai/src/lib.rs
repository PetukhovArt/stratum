//! Rhai-script plugin runtime for Stratum custom rules.

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod engine;
pub mod loader;
pub mod module_view;
pub mod rule;
pub mod watcher;

pub use engine::build_engine;
pub use loader::{LoadError, PLUGIN_ID_FLOOR, load};
pub use module_view::{ModuleView, build_view};
pub use rule::RhaiRule;
pub use watcher::{PluginChange, WatchError, watch_plugins};
