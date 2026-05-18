//! Rule trait + registry + the five baseline Stratum rules.

#![forbid(unsafe_code)]

pub mod ids;
pub mod rule;
pub mod scope;

pub use ids::{
    DEPTH_RATIO, MILLER_LIMIT, NO_CIRCULAR_DEPS, NO_CROSS_LAYER_IMPORT, STAGE_PURITY, id_for,
    slug_for,
};
pub use rule::{EmptyOptions, Rule};
pub use scope::{ProjectScope, RuleScope};
