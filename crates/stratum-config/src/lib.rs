//! JSONC-based config + override resolution + JSON Schema generation for Stratum.

#![forbid(unsafe_code)]

pub mod config;
pub mod error;
mod extends;
pub mod overrides;
pub mod parse;
mod presets;
pub mod schema;

pub use config::{Config, LayerConfig, OverrideBlock, ProjectConfig, RuleConfig};
pub use error::ConfigError;
pub use extends::config_files;
pub use overrides::{EffectiveRules, resolve_for_file};
pub use parse::{parse_file, parse_str};
pub use presets::recommended_rules;
pub use schema::schema;
