//! Type-erased rule registry. Phase 4 will wrap each entry in a Salsa query.

use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::rule::Rule;
use crate::scope::RuleScope;

/// Type-erased wrapper around a `Rule` impl. Owned by [`RuleRegistry`].
pub trait DynRule: Send + Sync {
    fn id(&self) -> RuleId;
    fn slug(&self) -> &'static str;
    fn default_severity(&self) -> Severity;
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

    fn run(
        &self,
        graph: &CompoundGraph,
        severity: Severity,
        options: &serde_json::Value,
    ) -> Vec<Violation> {
        let opts: R::Options = if options.is_null() {
            R::Options::default()
        } else {
            serde_json::from_value(options.clone()).unwrap_or_default()
        };
        let mut out = Vec::new();
        for scope in R::Scope::enumerate(graph) {
            out.extend(self.inner.check(graph, scope, &opts, severity));
        }
        out
    }
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

    /// All five baseline Stratum rules pre-registered.
    #[must_use]
    pub fn with_builtins() -> Self {
        use crate::builtin::{
            depth_ratio::DepthRatio, miller_limit::MillerLimit, no_circular_deps::NoCircularDeps,
            no_cross_layer_import::NoCrossLayerImport, stage_purity::StagePurity,
        };
        let mut r = Self::new();
        r.register(NoCrossLayerImport);
        r.register(NoCircularDeps);
        r.register(StagePurity);
        r.register(MillerLimit);
        r.register(DepthRatio);
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_registry_has_five_rules() {
        let r = RuleRegistry::with_builtins();
        assert_eq!(r.rules().len(), 5);
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
