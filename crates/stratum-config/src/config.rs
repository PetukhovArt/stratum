use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stratum_core::severity::Severity;

/// The whole `stratum.config.jsonc` document.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub project: ProjectConfig,
    /// Merged with the `extends` parent by `id`: a child entry overrides the parent entry field by field.
    #[serde(default)]
    pub layers: Vec<LayerConfig>,
    /// Keyed by rule slug. Merged with the `extends` parent by slug, field by field.
    #[serde(default)]
    pub rules: BTreeMap<String, RuleConfig>,
    /// Appended after the `extends` parent's overrides; the last matching block wins per rule.
    #[serde(default)]
    pub overrides: Vec<OverrideBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    #[schemars(with = "String")]
    pub root: Utf8PathBuf,
    /// A config file path relative to this file, or a built-in preset: `stratum:recommended`, `stratum:fsd`.
    #[serde(default)]
    pub extends: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LayerConfig {
    pub id: String,
    /// Directory relative to the project root; a module belongs to the layer with the longest matching path.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    /// Ids of the layers this layer may import from.
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    pub severity: Severity,
    /// Rule-specific, validated against the rule when the config loads: unknown keys are errors.
    #[serde(default)]
    pub options: serde_json::Value,
    /// A `.rhai` script relative to the project root that implements this slug as a plugin rule.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub script: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OverrideBlock {
    /// Globs matched against forward-slash paths relative to the project root.
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
