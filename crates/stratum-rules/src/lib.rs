//! Rule trait + registry + the five baseline Stratum rules.

#![forbid(unsafe_code)]

pub mod builtin;
pub mod ids;
pub mod options;
pub mod rule;
pub mod scope;

pub use builtin::miller_limit::MillerLimit;
pub use builtin::no_circular_deps::NoCircularDeps;
pub use builtin::no_cross_layer_import::NoCrossLayerImport;
pub use builtin::stage_purity::StagePurity;
pub use options::{DepthRatioOptions, MillerLimitOptions};

pub use ids::{
    DEPTH_RATIO, MILLER_LIMIT, NO_CIRCULAR_DEPS, NO_CROSS_LAYER_IMPORT, STAGE_PURITY, id_for,
    slug_for,
};
pub use rule::{EmptyOptions, Rule};
pub use scope::{ProjectScope, RuleScope};
