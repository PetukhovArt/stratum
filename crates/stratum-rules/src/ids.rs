use stratum_core::ids::RuleId;

pub const NO_CROSS_LAYER_IMPORT: RuleId = RuleId::new(1);
pub const NO_CIRCULAR_DEPS: RuleId = RuleId::new(2);
pub const STAGE_PURITY: RuleId = RuleId::new(3);
pub const MILLER_LIMIT: RuleId = RuleId::new(4);
pub const DEPTH_RATIO: RuleId = RuleId::new(5);
pub const VISIBILITY_SCOPE: RuleId = RuleId::new(6);
pub const CROSS_ENTITY_PATTERN: RuleId = RuleId::new(7);
pub const DEEP_MODULE: RuleId = RuleId::new(8);
pub const PROMOTION_PRESSURE: RuleId = RuleId::new(9);
pub const INSTABILITY: RuleId = RuleId::new(10);

#[must_use]
pub fn slug_for(id: RuleId) -> Option<&'static str> {
    Some(match id {
        NO_CROSS_LAYER_IMPORT => "stratum/no-cross-layer-import",
        NO_CIRCULAR_DEPS => "stratum/no-circular-deps",
        STAGE_PURITY => "stratum/stage-purity",
        MILLER_LIMIT => "stratum/miller-limit",
        DEPTH_RATIO => "stratum/depth-ratio",
        VISIBILITY_SCOPE => "stratum/visibility-scope",
        CROSS_ENTITY_PATTERN => "stratum/cross-entity-pattern",
        DEEP_MODULE => "stratum/deep-module",
        PROMOTION_PRESSURE => "stratum/promotion-pressure",
        INSTABILITY => "stratum/instability",
        _ => return None,
    })
}

#[must_use]
pub fn id_for(slug: &str) -> Option<RuleId> {
    Some(match slug {
        "stratum/no-cross-layer-import" => NO_CROSS_LAYER_IMPORT,
        "stratum/no-circular-deps" => NO_CIRCULAR_DEPS,
        "stratum/stage-purity" => STAGE_PURITY,
        "stratum/miller-limit" => MILLER_LIMIT,
        "stratum/depth-ratio" => DEPTH_RATIO,
        "stratum/visibility-scope" => VISIBILITY_SCOPE,
        "stratum/cross-entity-pattern" => CROSS_ENTITY_PATTERN,
        "stratum/deep-module" => DEEP_MODULE,
        "stratum/promotion-pressure" => PROMOTION_PRESSURE,
        "stratum/instability" => INSTABILITY,
        _ => return None,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn slug_round_trip() {
        for id in [
            NO_CROSS_LAYER_IMPORT,
            NO_CIRCULAR_DEPS,
            STAGE_PURITY,
            MILLER_LIMIT,
            DEPTH_RATIO,
            VISIBILITY_SCOPE,
            CROSS_ENTITY_PATTERN,
            DEEP_MODULE,
            PROMOTION_PRESSURE,
            INSTABILITY,
        ] {
            let slug = slug_for(id).unwrap();
            assert_eq!(id_for(slug), Some(id));
        }
    }
}
