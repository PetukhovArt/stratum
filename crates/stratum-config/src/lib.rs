//! JSONC-based config + override resolution + JSON Schema generation for Stratum.

#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod parse;
pub mod schema;

pub use config::{Config, LayerConfig, OverrideBlock, ProjectConfig, RuleConfig};
pub use error::ConfigError;
pub use parse::{parse_file, parse_str};
pub use schema::schema;
