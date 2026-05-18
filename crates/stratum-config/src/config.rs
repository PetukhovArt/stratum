use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stratum_core::severity::Severity;

/// The whole `stratum.config.jsonc` document.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Config {
    pub version: u32,
    pub project: ProjectConfig,
    pub layers: Vec<LayerConfig>,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleConfig>,
    #[serde(default)]
    pub overrides: Vec<OverrideBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ProjectConfig {
    #[schemars(with = "String")]
    pub root: Utf8PathBuf,
    #[serde(default)]
    pub extends: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct LayerConfig {
    pub id: String,
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RuleConfig {
    pub severity: Severity,
    #[serde(default)]
    pub options: serde_json::Value,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub script: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OverrideBlock {
    pub files: Vec<String>,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleConfig>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn rule_config_severity_only_round_trips() {
        let r = RuleConfig {
            severity: Severity::Error,
            options: serde_json::Value::Null,
            script: None,
        };
        let j = serde_json::to_string(&r).unwrap();
        let back: RuleConfig = serde_json::from_str(&j).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn rule_config_with_options_round_trips() {
        let r = RuleConfig {
            severity: Severity::Warning,
            options: serde_json::json!({ "max_children": 7 }),
            script: None,
        };
        let j = serde_json::to_string(&r).unwrap();
        let back: RuleConfig = serde_json::from_str(&j).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn override_block_round_trips() {
        let o = OverrideBlock {
            files: vec!["src/legacy/**".into()],
            rules: BTreeMap::from([(
                "stratum/no-cross-layer-import".into(),
                RuleConfig {
                    severity: Severity::Off,
                    options: serde_json::Value::Null,
                    script: None,
                },
            )]),
        };
        let j = serde_json::to_string(&o).unwrap();
        let back: OverrideBlock = serde_json::from_str(&j).unwrap();
        assert_eq!(o, back);
    }
}
