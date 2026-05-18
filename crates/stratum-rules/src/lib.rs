//! Rule trait + registry + the five baseline Stratum rules.

#![forbid(unsafe_code)]

pub mod builtin;
pub mod config_hash;
pub mod ids;
pub mod options;
pub mod registry;
pub mod rule;
pub mod runner;
pub mod scope;

pub use builtin::depth_ratio::DepthRatio;
pub use builtin::miller_limit::MillerLimit;
pub use builtin::no_circular_deps::NoCircularDeps;
pub use builtin::no_cross_layer_import::NoCrossLayerImport;
pub use builtin::stage_purity::StagePurity;
pub use options::{DepthRatioOptions, MillerLimitOptions};

pub use config_hash::rule_config_hash;
pub use registry::{DynRule, DynRuleAdapter, RuleRegistry};
pub use runner::run_all;

pub use ids::{
    DEPTH_RATIO, MILLER_LIMIT, NO_CIRCULAR_DEPS, NO_CROSS_LAYER_IMPORT, STAGE_PURITY, id_for,
    slug_for,
};
pub use rule::{EmptyOptions, Rule};
pub use scope::{ProjectScope, RuleScope};
