use std::collections::BTreeMap;

use camino::Utf8Path;
use globset::{Glob, GlobSet, GlobSetBuilder};

use stratum_core::severity::Severity;

use crate::{Config, ConfigError, OverrideBlock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveRules {
    pub severities: BTreeMap<String, Severity>,
}

#[derive(Debug, Clone)]
pub struct Overrides<'c> {
    config: &'c Config,
    globs: GlobSet,
    block_of_glob: Vec<usize>,
}

impl<'c> Overrides<'c> {
    /// # Errors
    /// Returns [`ConfigError::BadGlob`] if any `overrides[].files` entry is not a
    /// valid glob.
    pub fn new(config: &'c Config) -> Result<Self, ConfigError> {
        let mut builder = GlobSetBuilder::new();
        let mut block_of_glob = Vec::new();
        for (block, o) in config.overrides.iter().enumerate() {
            for pattern in &o.files {
                let glob = Glob::new(pattern).map_err(|e| ConfigError::BadGlob {
                    pattern: pattern.clone(),
                    source: e,
                })?;
                builder.add(glob);
                block_of_glob.push(block);
            }
        }
        let globs = builder.build().map_err(|e| ConfigError::BadGlob {
            pattern: config
                .overrides
                .iter()
                .flat_map(|o| o.files.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(","),
            source: e,
        })?;
        Ok(Self {
            config,
            globs,
            block_of_glob,
        })
    }

    /// The blocks whose `files` match the root-relative, `/`-separated `file`, in config order.
    #[must_use]
    pub fn matching(&self, file: &str) -> Vec<&'c OverrideBlock> {
        let mut indices: Vec<usize> = self
            .globs
            .matches(file)
            .into_iter()
            .map(|g| self.block_of_glob[g])
            .collect();
        indices.sort_unstable();
        indices.dedup();
        indices
            .into_iter()
            .map(|b| &self.config.overrides[b])
            .collect()
    }

    /// The configured severities for `file`, the last matching block winning per rule.
    #[must_use]
    pub fn resolve(&self, file: &Utf8Path) -> EffectiveRules {
        let mut severities: BTreeMap<String, Severity> = self
            .config
            .rules
            .iter()
            .map(|(slug, rc)| (slug.clone(), rc.severity))
            .collect();
        for block in self.matching(file.as_str()) {
            for (slug, rule) in &block.rules {
                severities.insert(slug.clone(), rule.severity);
            }
        }
        EffectiveRules { severities }
    }
}

pub(crate) fn validate(config: &Config) -> Result<(), ConfigError> {
    Overrides::new(config).map(drop)
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
                entry_points: vec![],
            },
            layers: vec![LayerConfig {
                id: "shared".into(),
                path: "src/shared".into(),
                depends_on: vec![],
                slices: false,
            }],
            rules,
            overrides: vec![],
            features: std::collections::BTreeMap::new(),
            systems: vec![],
        }
    }

    #[test]
    fn no_overrides_returns_base() {
        let cfg = base();
        let eff = Overrides::new(&cfg)
            .unwrap()
            .resolve(Utf8Path::new("src/app/main.ts"));
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
        let eff = Overrides::new(&cfg)
            .unwrap()
            .resolve(Utf8Path::new("src/legacy/old.ts"));
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
        let eff = Overrides::new(&cfg)
            .unwrap()
            .resolve(Utf8Path::new("src/legacy/old.ts"));
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
        let eff = Overrides::new(&cfg)
            .unwrap()
            .resolve(Utf8Path::new("src/legacy/old.ts"));
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
        let eff = Overrides::new(&cfg)
            .unwrap()
            .resolve(Utf8Path::new("src/x/__tests__/y.ts"));
        assert_eq!(eff.severities["stratum/miller-limit"], Severity::Off);
    }
}
