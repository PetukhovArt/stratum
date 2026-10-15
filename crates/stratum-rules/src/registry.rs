//! Type-erased rule registry. Phase 4 will wrap each entry in a Salsa query.

use stratum_config::{Config, ConfigError};
use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::docs::RuleDoc;
use crate::rule::Rule;
use crate::scope::RuleScope;

/// Type-erased wrapper around a `Rule` impl. Owned by [`RuleRegistry`].
pub trait DynRule: Send + Sync {
    fn id(&self) -> RuleId;
    fn slug(&self) -> &'static str;
    fn default_severity(&self) -> Severity;
    fn doc(&self) -> Option<RuleDoc>;
    /// # Errors
    /// A message naming the offending option key when `options` do not fit the rule.
    fn check_options(&self, options: &serde_json::Value) -> Result<(), String>;
    fn run(
        &self,
        graph: &CompoundGraph,
        severity: Severity,
        options: &serde_json::Value,
    ) -> Vec<Violation>;
}

#[derive(Debug)]
pub struct DynRuleAdapter<R: Rule> {
    inner: R,
}

impl<R: Rule> DynRuleAdapter<R> {
    pub const fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: Rule> DynRule for DynRuleAdapter<R> {
    fn id(&self) -> RuleId {
        self.inner.id()
    }
    fn slug(&self) -> &'static str {
        self.inner.slug()
    }
    fn default_severity(&self) -> Severity {
        self.inner.default_severity()
    }
    fn doc(&self) -> Option<RuleDoc> {
        self.inner.doc()
    }

    fn check_options(&self, options: &serde_json::Value) -> Result<(), String> {
        parse_options::<R::Options>(options).map(drop)
    }

    fn run(
        &self,
        graph: &CompoundGraph,
        severity: Severity,
        options: &serde_json::Value,
    ) -> Vec<Violation> {
        let opts: R::Options = parse_options(options).unwrap_or_default();
        let mut out = Vec::new();
        for scope in R::Scope::enumerate(graph) {
            out.extend(self.inner.check(graph, scope, &opts, severity));
        }
        out
    }
}

fn parse_options<T: serde::de::DeserializeOwned + Default>(
    options: &serde_json::Value,
) -> Result<T, String> {
    if options.is_null() {
        return Ok(T::default());
    }
    serde_json::from_value(options.clone()).map_err(|e| {
        let bad_key = options.as_object().and_then(|fields| {
            fields.iter().find(|(key, value)| {
                serde_json::from_value::<T>(serde_json::json!({ key.as_str(): value })).is_err()
            })
        });
        match bad_key {
            Some((key, _)) => format!("option {key:?}: {e}"),
            None => format!("options: {e}"),
        }
    })
}

#[derive(Default)]
pub struct RuleRegistry {
    rules: Vec<Box<dyn DynRule>>,
}

impl std::fmt::Debug for RuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleRegistry")
            .field("count", &self.rules.len())
            .finish()
    }
}

impl RuleRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<R: Rule + 'static>(&mut self, rule: R) {
        self.rules.push(Box::new(DynRuleAdapter::new(rule)));
    }

    #[must_use]
    pub fn rules(&self) -> &[Box<dyn DynRule>] {
        &self.rules
    }

    #[must_use]
    pub fn by_slug(&self, slug: &str) -> Option<&dyn DynRule> {
        self.rules
            .iter()
            .find(|r| r.slug() == slug)
            .map(AsRef::as_ref)
    }

    #[must_use]
    pub fn by_id(&self, id: RuleId) -> Option<&dyn DynRule> {
        self.rules.iter().find(|r| r.id() == id).map(AsRef::as_ref)
    }

    /// # Errors
    /// [`ConfigError::Rule`] naming the slug, and the option key when one is at fault,
    /// for an unknown slug in `rules` or `overrides` or options its rule rejects.
    pub fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        let entries = config
            .rules
            .iter()
            .chain(config.overrides.iter().flat_map(|o| o.rules.iter()));
        for (slug, rule_config) in entries {
            let Some(rule) = self.by_slug(slug) else {
                return Err(ConfigError::Rule {
                    slug: slug.clone(),
                    message: "unknown rule; run `stratum-lint rules` for the list".into(),
                });
            };
            rule.check_options(&rule_config.options)
                .map_err(|message| ConfigError::Rule {
                    slug: slug.clone(),
                    message,
                })?;
        }
        Ok(())
    }

    /// All ten Stratum rules pre-registered (5 Phase 3 baseline + 5 Phase 5 bundled).
    #[must_use]
    pub fn with_builtins() -> Self {
        use crate::builtin::{
            cross_entity_pattern::CrossEntityPattern, deep_module::DeepModule,
            depth_ratio::DepthRatio, import_policy::ImportPolicy, miller_limit::MillerLimit,
            no_circular_deps::NoCircularDeps, no_cross_layer_import::NoCrossLayerImport,
            promotion_pressure::PromotionPressure, purity::PurityRule,
            visibility_scope::VisibilityScope,
        };
        let mut r = Self::new();
        r.register(NoCrossLayerImport);
        r.register(NoCircularDeps);
        r.register(PurityRule);
        r.register(MillerLimit);
        r.register(DepthRatio);
        r.register(VisibilityScope);
        r.register(CrossEntityPattern);
        r.register(DeepModule);
        r.register(PromotionPressure);
        r.register(ImportPolicy);
        r
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn builtins_registry_has_ten_rules() {
        let r = RuleRegistry::with_builtins();
        assert_eq!(r.rules().len(), 10);
    }

    fn validate(rules: &serde_json::Value) -> Result<(), ConfigError> {
        let config: Config = serde_json::from_value(serde_json::json!({
            "version": 1, "project": { "root": "." }, "rules": rules,
        }))
        .unwrap();
        RuleRegistry::with_builtins().validate(&config)
    }

    #[test]
    fn misspelled_option_key_is_an_error_naming_slug_and_key() {
        let err = validate(&serde_json::json!({
            "stratum/miller-limit": { "severity": "warning", "options": { "max_child": 3 } },
        }))
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("stratum/miller-limit") && err.contains("max_child"),
            "{err}"
        );
    }

    #[test]
    fn mistyped_option_value_names_the_key() {
        let err = validate(&serde_json::json!({
            "stratum/miller-limit": { "severity": "warning", "options": { "max_children": "7" } },
        }))
        .unwrap_err()
        .to_string();
        assert!(err.contains("\"max_children\""), "{err}");
    }

    #[test]
    fn option_on_an_optionless_rule_and_unknown_slug_are_errors() {
        let err = validate(&serde_json::json!({
            "stratum/no-circular-deps": { "severity": "error", "options": { "depth": 2 } },
        }))
        .unwrap_err()
        .to_string();
        assert!(err.contains("depth"), "{err}");
        assert!(
            validate(&serde_json::json!({ "stratum/no-such-rule": { "severity": "error" } }))
                .is_err()
        );
        assert!(validate(&serde_json::json!({ "stratum/no-circular-deps": { "severity": "error", "options": {} } })).is_ok());
    }

    #[test]
    fn slugs_unique_within_builtins() {
        let r = RuleRegistry::with_builtins();
        let mut slugs: Vec<&str> = r.rules().iter().map(|rl| rl.slug()).collect();
        slugs.sort_unstable();
        let n = slugs.len();
        slugs.dedup();
        assert_eq!(slugs.len(), n);
    }
}
