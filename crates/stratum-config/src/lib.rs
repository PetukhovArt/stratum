#![forbid(unsafe_code)]

pub mod config;
mod entry_points;
pub mod error;
mod extends;
mod features;
mod ignore;
pub mod overrides;
pub mod parse;
mod presets;
pub mod schema;
mod systems;

pub use config::{Config, LayerConfig, OverrideBlock, OverrideRule, ProjectConfig, RuleConfig};
pub use entry_points::{EntryPoints, literal_dir};
pub use error::ConfigError;
pub use extends::config_files;
pub use features::FeatureConfig;
pub use ignore::{DEFAULT_IGNORE, IgnoreSet};
pub use overrides::{EffectiveRules, Overrides};
pub use parse::{parse_file, parse_str};
pub use presets::{recommended_rules, recommended_structure};
pub use schema::schema;
pub use systems::{CallMatcher, ContractSource, SystemConfig};
