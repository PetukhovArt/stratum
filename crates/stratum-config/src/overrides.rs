use std::collections::BTreeMap;

use camino::Utf8Path;
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::{Config, ConfigError, RuleConfig};

/// The effective rule set for one source file, after base rules and any
/// matching overrides are merged in array order (last-matching-wins per rule key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveRules {
    pub rules: BTreeMap<String, RuleConfig>,
}

/// Resolve the effective rule set for `file` against `config`.
///
/// # Errors
/// Returns [`ConfigError::BadGlob`] if any `overrides[].files` entry is not a
/// valid glob.
pub fn resolve_for_file(config: &Config, file: &Utf8Path) -> Result<EffectiveRules, ConfigError> {
    let mut effective = config.rules.clone();
    for block in &config.overrides {
        let mut builder = GlobSetBuilder::new();
        for pat in &block.files {
            let glob = Glob::new(pat).map_err(|e| ConfigError::BadGlob {
                pattern: pat.clone(),
                source: e,
            })?;
            builder.add(glob);
        }
        let set: GlobSet = builder.build().map_err(|e| ConfigError::BadGlob {
            pattern: block.files.join(","),
            source: e,
        })?;
        if set.is_match(file.as_str()) {
            for (rule_id, rc) in &block.rules {
                effective.insert(rule_id.clone(), rc.clone());
            }
        }
    }
    Ok(EffectiveRules { rules: effective })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{LayerConfig, OverrideBlock, ProjectConfig};
    use stratum_core::severity::Severity;

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
            project: ProjectConfig {
                root: "./src".into(),
                extends: None,
            },
            layers: vec![LayerConfig {
                id: "shared".into(),
                path: "src/shared".into(),
                depends_on: vec![],
            }],
            rules,
            overrides: vec![],
        }
    }

    #[test]
    fn no_overrides_returns_base() {
        let cfg = base();
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/app/main.ts")).unwrap();
        assert_eq!(eff.rules.len(), 2);
        assert_eq!(
            eff.rules["stratum/no-cross-layer-import"].severity,
            Severity::Error
        );
    }

    #[test]
    fn single_matching_override_replaces_entry() {
        let mut cfg = base();
        let mut rules = BTreeMap::new();
        rules.insert("stratum/no-cross-layer-import".into(), rc(Severity::Off));
        cfg.overrides.push(OverrideBlock {
            files: vec!["src/legacy/**".into()],
            rules,
        });
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/legacy/old.ts")).unwrap();
        assert_eq!(
            eff.rules["stratum/no-cross-layer-import"].severity,
            Severity::Off
        );
        assert_eq!(
            eff.rules["stratum/miller-limit"].severity,
            Severity::Warning
        );
    }

    #[test]
    fn non_matching_override_is_ignored() {
        let mut cfg = base();
        let mut rules = BTreeMap::new();
        rules.insert("stratum/no-cross-layer-import".into(), rc(Severity::Off));
        cfg.overrides.push(OverrideBlock {
            files: vec!["src/other/**".into()],
            rules,
        });
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/legacy/old.ts")).unwrap();
        assert_eq!(
            eff.rules["stratum/no-cross-layer-import"].severity,
            Severity::Error
        );
    }

    #[test]
    fn last_matching_override_wins_per_rule_key() {
        let mut cfg = base();
        let mut a = BTreeMap::new();
        a.insert(
            "stratum/no-cross-layer-import".into(),
            rc(Severity::Warning),
        );
        cfg.overrides.push(OverrideBlock {
            files: vec!["src/legacy/**".into()],
            rules: a,
        });
        let mut b = BTreeMap::new();
        b.insert("stratum/no-cross-layer-import".into(), rc(Severity::Off));
        cfg.overrides.push(OverrideBlock {
            files: vec!["src/legacy/**".into()],
            rules: b,
        });
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/legacy/old.ts")).unwrap();
        assert_eq!(
            eff.rules["stratum/no-cross-layer-import"].severity,
            Severity::Off
        );
    }

    #[test]
    fn multi_pattern_block_matches_any() {
        let mut cfg = base();
        let mut rules = BTreeMap::new();
        rules.insert("stratum/miller-limit".into(), rc(Severity::Off));
        cfg.overrides.push(OverrideBlock {
            files: vec!["**/*.test.ts".into(), "**/__tests__/**".into()],
            rules,
        });
        let eff = resolve_for_file(&cfg, Utf8Path::new("src/x/__tests__/y.ts")).unwrap();
        assert_eq!(eff.rules["stratum/miller-limit"].severity, Severity::Off);
    }
}
