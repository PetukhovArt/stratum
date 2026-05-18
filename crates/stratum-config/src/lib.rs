//! JSONC-based config + override resolution + JSON Schema generation for Stratum.

#![forbid(unsafe_code)]

pub mod config;
pub mod error;

pub use config::{Config, LayerConfig, OverrideBlock, ProjectConfig, RuleConfig};
pub use error::ConfigError;
