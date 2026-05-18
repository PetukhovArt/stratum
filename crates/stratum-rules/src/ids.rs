use stratum_core::ids::RuleId;

pub const NO_CROSS_LAYER_IMPORT: RuleId = RuleId::new(1);
pub const NO_CIRCULAR_DEPS: RuleId = RuleId::new(2);
pub const STAGE_PURITY: RuleId = RuleId::new(3);
pub const MILLER_LIMIT: RuleId = RuleId::new(4);
pub const DEPTH_RATIO: RuleId = RuleId::new(5);

#[must_use]
pub fn slug_for(id: RuleId) -> Option<&'static str> {
    Some(match id {
        NO_CROSS_LAYER_IMPORT => "stratum/no-cross-layer-import",
        NO_CIRCULAR_DEPS => "stratum/no-circular-deps",
        STAGE_PURITY => "stratum/stage-purity",
        MILLER_LIMIT => "stratum/miller-limit",
        DEPTH_RATIO => "stratum/depth-ratio",
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
        ] {
            let slug = slug_for(id).unwrap();
            assert_eq!(id_for(slug), Some(id));
        }
    }
}
