use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::scope::RuleScope;

/// The contract every Stratum rule implements.
pub trait Rule: Send + Sync {
    type Scope: RuleScope;
    type Options: serde::de::DeserializeOwned + Default + Clone + Send + Sync + 'static;

    fn id(&self) -> RuleId;
    fn slug(&self) -> &'static str;
    fn default_severity(&self) -> Severity;

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: Self::Scope,
        options: &Self::Options,
        severity: Severity,
    ) -> Vec<Violation>;
}

/// Rules with no options.
#[derive(Debug, Default, Clone, Copy, serde::Deserialize)]
pub struct EmptyOptions;
