use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stratum_core::{severity::Severity, structure::Structure};

/// The whole `stratum.config.jsonc` document.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub project: ProjectConfig,
    /// Merged with the `extends` parent field by field; a child list replaces the parent's list.
    #[serde(default)]
    pub structure: Structure,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// A config file path relative to this file, or a built-in preset: `stratum:recommended`, `stratum:fsd`.
    #[serde(default)]
    pub extends: Option<String>,
    /// Globs relative to the project root, on top of the built-in defaults (`node_modules`,
    /// `dist`, `out`, `build`, `coverage`, `.git`, `*.d.ts`). Matching files and directories are
    /// never walked or parsed; imports of them make no edge. `!glob` un-ignores; the last matching
    /// glob wins. Appended after the `extends` parent's list.
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Globs relative to the project root of the files the program starts from: the roots of
    /// reachability for `stratum/unused-exports` and `stratum/unreachable-modules`. Required when
    /// either rule is on; each glob must match a module. A child list replaces the `extends`
    /// parent's list.
    #[serde(default)]
    pub entry_points: Vec<String>,
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
    /// Each direct child of `path`, folder or source file, is a slice: a container isolated from its siblings.
    #[serde(default)]
    pub slices: bool,
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
    pub rules: BTreeMap<String, OverrideRule>,
}

/// Severity only: options and scripts come from the base `rules` entry.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OverrideRule {
    pub severity: Severity,
}
