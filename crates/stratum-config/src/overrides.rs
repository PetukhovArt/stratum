use std::collections::BTreeMap;

use camino::Utf8Path;
use globset::{Glob, GlobSet, GlobSetBuilder};

use stratum_core::severity::Severity;

use crate::{Config, ConfigError, OverrideBlock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveRules {
    pub severities: BTreeMap<String, Severity>,
}

/// # Errors
/// Returns [`ConfigError::BadGlob`] if any `overrides[].files` entry is not a
/// valid glob.
pub fn resolve_for_file(config: &Config, file: &Utf8Path) -> Result<EffectiveRules, ConfigError> {
    let mut severities: BTreeMap<String, Severity> = config
        .rules
        .iter()
        .map(|(slug, rc)| (slug.clone(), rc.severity))
        .collect();
    for block in &config.overrides {
        if globs(block)?.is_match(file.as_str()) {
            for (slug, rule) in &block.rules {
                severities.insert(slug.clone(), rule.severity);
            }
        }
    }
    Ok(EffectiveRules { severities })
}

pub(crate) fn validate(config: &Config) -> Result<(), ConfigError> {
    config.overrides.iter().try_for_each(|b| globs(b).map(drop))
}

fn globs(block: &OverrideBlock) -> Result<GlobSet, ConfigError> {
    let mut builder = GlobSetBuilder::new();
    for pat in &block.files {
        let glob = Glob::new(pat).map_err(|e| ConfigError::BadGlob {
            pattern: pat.clone(),
            source: e,
        })?;
        builder.add(glob);
    }
    builder.build().map_err(|e| ConfigError::BadGlob {
        pattern: block.files.join(","),
        source: e,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{LayerConfig, OverrideRule, ProjectConfig, RuleConfig};

    fn rc(severity: Severity) -> RuleConfig {
        RuleConfig {
            severity,
            options: serde_json::Value::Null,
            script: None,
        }
    }

    fn base() -> Config {
        let mut rules = BTreeMap::new();
        rules.insert("stratum/no-cross-layer-import".into(), rc(Severity::Error));
        rules.insert("stratum/miller-limit".into(), rc(Severity::Warning));
        Config {
            version: 1,
            structure: stratum_core::structure::Structure::default(),
            project: ProjectConfig {
                extends: None,
                ignore: vec![],
            },
            layers: vec![LayerConfig {
                id: "shared".into(),
                path: "src/shared".into(),
                depends_on: vec![],
                slices: false,
            }],
            rules,
            overrides: vec![],
        }
    }

    #[test]
    fn no_overrides_returns_base() {
        let cfg = base();
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/app/main.ts")).unwrap();
        assert_eq!(eff.severities.len(), 2);
        assert_eq!(
            eff.severities["stratum/no-cross-layer-import"],
            Severity::Error
        );
    }

    #[test]
    fn single_matching_override_replaces_entry() {
        let mut cfg = base();
        let mut rules = BTreeMap::new();
        rules.insert(
            "stratum/no-cross-layer-import".into(),
            OverrideRule {
                severity: Severity::Off,
            },
        );
        cfg.overrides.push(OverrideBlock {
            files: vec!["src/legacy/**".into()],
            rules,
        });
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/legacy/old.ts")).unwrap();
        assert_eq!(
            eff.severities["stratum/no-cross-layer-import"],
            Severity::Off
        );
        assert_eq!(eff.severities["stratum/miller-limit"], Severity::Warning);
    }

    #[test]
    fn non_matching_override_is_ignored() {
        let mut cfg = base();
        let mut rules = BTreeMap::new();
        rules.insert(
            "stratum/no-cross-layer-import".into(),
            OverrideRule {
                severity: Severity::Off,
            },
        );
        cfg.overrides.push(OverrideBlock {
            files: vec!["src/other/**".into()],
            rules,
        });
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/legacy/old.ts")).unwrap();
        assert_eq!(
            eff.severities["stratum/no-cross-layer-import"],
            Severity::Error
        );
    }

    #[test]
    fn last_matching_override_wins_per_rule_key() {
        let mut cfg = base();
        let mut a = BTreeMap::new();
        a.insert(
            "stratum/no-cross-layer-import".into(),
            OverrideRule {
                severity: Severity::Warning,
            },
        );
        cfg.overrides.push(OverrideBlock {
            files: vec!["src/legacy/**".into()],
            rules: a,
        });
        let mut b = BTreeMap::new();
        b.insert(
            "stratum/no-cross-layer-import".into(),
            OverrideRule {
                severity: Severity::Off,
            },
        );
        cfg.overrides.push(OverrideBlock {
            files: vec!["src/legacy/**".into()],
            rules: b,
        });
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/legacy/old.ts")).unwrap();
        assert_eq!(
            eff.severities["stratum/no-cross-layer-import"],
            Severity::Off
        );
    }

    #[test]
    fn multi_pattern_block_matches_any() {
        let mut cfg = base();
        let mut rules = BTreeMap::new();
        rules.insert(
            "stratum/miller-limit".into(),
            OverrideRule {
                severity: Severity::Off,
            },
        );
        cfg.overrides.push(OverrideBlock {
            files: vec!["**/*.test.ts".into(), "**/__tests__/**".into()],
            rules,
        });
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/x/__tests__/y.ts")).unwrap();
        assert_eq!(eff.severities["stratum/miller-limit"], Severity::Off);
    }
}
