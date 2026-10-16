#![forbid(unsafe_code)]

pub mod builtin;
pub mod docs;
pub mod ids;
pub mod options;
mod paths;
pub mod registry;
pub mod rule;
pub mod runner;
pub mod scope;
mod severity;
pub mod territory;

pub use builtin::container_breadth::ContainerBreadth;
pub use builtin::deep_module::DeepModule;
pub use builtin::depth_ratio::DepthRatio;
pub use builtin::import_policy::{ImportPolicy, ImportPolicyOptions};
pub use builtin::miller_limit::MillerLimit;
pub use builtin::no_circular_deps::NoCircularDeps;
pub use builtin::no_cross_layer_import::NoCrossLayerImport;
pub use builtin::promotion_pressure::PromotionPressure;
pub use builtin::public_api::PublicApi;
pub use builtin::purity::PurityRule;
pub use builtin::restricted_calls::{RestrictedCalls, RestrictedCallsOptions, tracked_calls};
pub use builtin::sibling_isolation::SiblingIsolation;
pub use builtin::slice_fan_in::SliceFanIn;
pub use builtin::stage_mixing::StageMixing;
pub use builtin::visibility_scope::VisibilityScope as VisibilityScopeRule;
pub use options::{
    ContainerBreadthOptions, DeepModuleOptions, DepthRatioOptions, MillerLimitOptions,
    PromotionPressureOptions, SliceFanInOptions,
};

pub use docs::RuleDoc;
pub use registry::{DynRule, DynRuleAdapter, RuleRegistry};
pub use runner::{run_all, run_all_with_registry};

pub use ids::{
    CONTAINER_BREADTH, DEEP_MODULE, DEPTH_RATIO, IMPORT_POLICY, MILLER_LIMIT, NO_CIRCULAR_DEPS,
    NO_CROSS_LAYER_IMPORT, PROMOTION_PRESSURE, PUBLIC_API, PURITY, RESTRICTED_CALLS,
    SIBLING_ISOLATION, SLICE_FAN_IN, STAGE_MIXING, VISIBILITY_SCOPE,
};
pub use rule::{EmptyOptions, Rule};
pub use scope::{ProjectScope, RuleScope};
