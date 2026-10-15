use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use stratum_core::severity::Severity;

use crate::RuleConfig;

pub const PREFIX: &str = "stratum:";
const RECOMMENDED: &str = "stratum:recommended";
const FSD: &str = "stratum:fsd";
pub const NAMES: [&str; 2] = [RECOMMENDED, FSD];

const RECOMMENDED_RULES: [(&str, Severity); 5] = [
    ("stratum/no-cross-layer-import", Severity::Warning),
    ("stratum/no-circular-deps", Severity::Error),
    ("stratum/stage-purity", Severity::Warning),
    ("stratum/miller-limit", Severity::Info),
    ("stratum/depth-ratio", Severity::Info),
];

const FSD_LAYERS: [&str; 6] = ["app", "pages", "widgets", "features", "entities", "shared"];

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

pub fn preset(name: &str) -> Option<Value> {
    match name {
        RECOMMENDED => {
            let rules: Map<String, Value> = RECOMMENDED_RULES
                .into_iter()
                .map(|(slug, severity)| {
                    (slug.to_owned(), json!({ "severity": severity.to_string() }))
                })
                .collect();
            Some(json!({ "rules": rules }))
        }
        FSD => {
            let layers: Vec<Value> = FSD_LAYERS
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    json!({ "id": id, "path": format!("src/{id}"), "depends_on": &FSD_LAYERS[i + 1..] })
                })
                .collect();
            Some(json!({ "project": { "extends": RECOMMENDED }, "layers": layers }))
        }
        _ => None,
    }
}
