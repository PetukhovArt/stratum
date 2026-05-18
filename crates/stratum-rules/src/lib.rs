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

pub use builtin::cross_entity_pattern::CrossEntityPattern;
pub use builtin::deep_module::DeepModule;
pub use builtin::depth_ratio::DepthRatio;
pub use builtin::miller_limit::MillerLimit;
pub use builtin::no_circular_deps::NoCircularDeps;
pub use builtin::no_cross_layer_import::NoCrossLayerImport;
pub use builtin::promotion_pressure::PromotionPressure;
pub use builtin::stage_purity::StagePurity;
pub use builtin::visibility_scope::VisibilityScope as VisibilityScopeRule;
pub use options::{
    CrossEntityPatternOptions, DeepModuleOptions, DepthRatioOptions, MillerLimitOptions,
    PromotionPressureOptions,
};

pub use config_hash::rule_config_hash;
pub use registry::{DynRule, DynRuleAdapter, RuleRegistry};
pub use runner::{run_all, run_all_with_registry};

pub use ids::{
    CROSS_ENTITY_PATTERN, DEEP_MODULE, DEPTH_RATIO, MILLER_LIMIT, NO_CIRCULAR_DEPS,
    NO_CROSS_LAYER_IMPORT, PROMOTION_PRESSURE, STAGE_PURITY, VISIBILITY_SCOPE, id_for, slug_for,
};
pub use rule::{EmptyOptions, Rule};
pub use scope::{ProjectScope, RuleScope};
