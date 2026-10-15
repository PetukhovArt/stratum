use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use stratum_core::severity::Severity;

use crate::RuleConfig;

pub const PREFIX: &str = "stratum:";
const RECOMMENDED: &str = "stratum:recommended";
const FSD: &str = "stratum:fsd";
pub const NAMES: [&str; 2] = [RECOMMENDED, FSD];

const RECOMMENDED_RULES: [(&str, Severity); 11] = [
    ("stratum/no-cross-layer-import", Severity::Warning),
    ("stratum/no-circular-deps", Severity::Error),
    ("stratum/purity", Severity::Warning),
    ("stratum/miller-limit", Severity::Info),
    ("stratum/depth-ratio", Severity::Info),
    ("stratum/visibility-scope", Severity::Error),
    ("stratum/public-api", Severity::Warning),
    ("stratum/sibling-isolation", Severity::Warning),
    ("stratum/restricted-calls", Severity::Off),
    ("stratum/stage-mixing", Severity::Off),
    ("stratum/container-breadth", Severity::Info),
];

const FSD_RULES: [(&str, Severity); 7] = [
    ("stratum/visibility-scope", Severity::Error),
    ("stratum/public-api", Severity::Error),
    ("stratum/sibling-isolation", Severity::Error),
    ("stratum/restricted-calls", Severity::Off),
    ("stratum/stage-mixing", Severity::Error),
    ("stratum/container-breadth", Severity::Warning),
    ("stratum/slice-fan-in", Severity::Warning),
];

const FSD_LAYERS: [&str; 6] = ["app", "pages", "widgets", "features", "entities", "shared"];
const FSD_SLICED_LAYERS: [&str; 4] = ["pages", "widgets", "features", "entities"];
const FSD_SEGMENTS: [&str; 5] = ["model", "ui", "api", "lib", "config"];

#[must_use]
pub fn recommended_rules() -> BTreeMap<String, RuleConfig> {
    RECOMMENDED_RULES
        .into_iter()
        .map(|(slug, severity)| {
            (
                slug.to_owned(),
                RuleConfig {
                    severity,
                    options: Value::Null,
                    script: None,
                },
            )
        })
        .collect()
}

fn rules_json(rules: &[(&str, Severity)]) -> Map<String, Value> {
    rules
        .iter()
        .map(|(slug, severity)| {
            (
                (*slug).to_owned(),
                json!({ "severity": severity.to_string() }),
            )
        })
        .collect()
}

pub fn preset(name: &str) -> Option<Value> {
    match name {
        RECOMMENDED => Some(json!({ "rules": rules_json(&RECOMMENDED_RULES) })),
        FSD => {
            let layers: Vec<Value> = FSD_LAYERS
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    json!({
                        "id": id,
                        "path": format!("src/{id}"),
                        "depends_on": &FSD_LAYERS[i + 1..],
                        "slices": FSD_SLICED_LAYERS.contains(id),
                    })
                })
                .collect();
            Some(json!({
                "project": { "extends": RECOMMENDED },
                "structure": { "segments": FSD_SEGMENTS },
                "layers": layers,
                "rules": rules_json(&FSD_RULES),
            }))
        }
        _ => None,
    }
}
