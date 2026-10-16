use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::builtin::edge::EdgeJudge;
use crate::docs::RuleDoc;
use crate::scope::RuleScope;

pub trait Rule: Send + Sync {
    type Scope: RuleScope;
    type Options: serde::de::DeserializeOwned + Default + Clone + Send + Sync + 'static;

    fn id(&self) -> RuleId;
    fn slug(&self) -> &'static str;
    fn default_severity(&self) -> Severity;

    fn doc(&self) -> Option<RuleDoc> {
        crate::docs::builtin(self.slug())
    }

    fn edge_judge(&self) -> Option<EdgeJudge<Self::Options>> {
        None
    }

    /// `severity` is the ceiling: stamp findings with it, or with a lower severity to downgrade
    /// them. The runner caps every finding at the configured severity and never raises one.
    fn check(
        &self,
        graph: &CompoundGraph,
        scope: Self::Scope,
        options: &Self::Options,
        severity: Severity,
    ) -> Vec<Violation>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EmptyOptions;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NoFields {}

impl<'de> serde::Deserialize<'de> for EmptyOptions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<NoFields>::deserialize(deserializer).map(|_| Self)
    }
}
