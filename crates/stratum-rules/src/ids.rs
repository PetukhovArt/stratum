use stratum_core::ids::RuleId;

pub const NO_CROSS_LAYER_IMPORT: RuleId = RuleId::new(1);
pub const NO_CIRCULAR_DEPS: RuleId = RuleId::new(2);
pub const PURITY: RuleId = RuleId::new(3);
pub const MILLER_LIMIT: RuleId = RuleId::new(4);
pub const DEPTH_RATIO: RuleId = RuleId::new(5);
pub const VISIBILITY_SCOPE: RuleId = RuleId::new(6);
pub const SLICE_FAN_IN: RuleId = RuleId::new(7);
pub const DEEP_MODULE: RuleId = RuleId::new(8);
pub const PROMOTION_PRESSURE: RuleId = RuleId::new(9);
pub const IMPORT_POLICY: RuleId = RuleId::new(10);
pub const PUBLIC_API: RuleId = RuleId::new(11);
pub const SIBLING_ISOLATION: RuleId = RuleId::new(12);
pub const RESTRICTED_CALLS: RuleId = RuleId::new(13);
pub const STAGE_MIXING: RuleId = RuleId::new(14);
pub const CONTAINER_BREADTH: RuleId = RuleId::new(15);
pub const ZONE_BREADTH: RuleId = RuleId::new(16);
pub const MISSING_ENTRY: RuleId = RuleId::new(17);
pub const NO_BARREL: RuleId = RuleId::new(18);
pub const COMPOSER_SCOPE: RuleId = RuleId::new(19);
pub const NAMING: RuleId = RuleId::new(20);
pub const MODULE_DOCS: RuleId = RuleId::new(21);
pub const UNUSED_EXPORTS: RuleId = RuleId::new(22);
pub const UNREACHABLE_MODULES: RuleId = RuleId::new(23);
pub const SUPPRESSION: RuleId = RuleId::new(24);

pub const BUILTIN: [RuleId; 24] = [
    NO_CROSS_LAYER_IMPORT,
    NO_CIRCULAR_DEPS,
    PURITY,
    MILLER_LIMIT,
    DEPTH_RATIO,
    VISIBILITY_SCOPE,
    SLICE_FAN_IN,
    DEEP_MODULE,
    PROMOTION_PRESSURE,
    IMPORT_POLICY,
    PUBLIC_API,
    SIBLING_ISOLATION,
    RESTRICTED_CALLS,
    STAGE_MIXING,
    CONTAINER_BREADTH,
    ZONE_BREADTH,
    MISSING_ENTRY,
    NO_BARREL,
    COMPOSER_SCOPE,
    NAMING,
    MODULE_DOCS,
    UNUSED_EXPORTS,
    UNREACHABLE_MODULES,
    SUPPRESSION,
];

#[must_use]
pub const fn max_builtin() -> u32 {
    let mut max = 0;
    let mut i = 0;
    while i < BUILTIN.len() {
        if BUILTIN[i].raw() > max {
            max = BUILTIN[i].raw();
        }
        i += 1;
    }
    max
}
