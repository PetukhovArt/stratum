use std::collections::HashSet;

use serde::Deserialize;
use stratum_config::Config;
use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::ids::RESTRICTED_CALLS;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct RestrictedCalls;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(transparent)]
pub struct RestrictedCallsOptions(pub Vec<CallPolicy>);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallPolicy {
    pub calls: Vec<String>,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub message: Option<String>,
}

impl Rule for RestrictedCalls {
    type Scope = ModuleId;
    type Options = RestrictedCallsOptions;

    fn id(&self) -> RuleId {
        RESTRICTED_CALLS
    }
    fn slug(&self) -> &'static str {
        "stratum/restricted-calls"
    }
    fn default_severity(&self) -> Severity {
        Severity::Off
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ModuleId,
        _options: &RestrictedCallsOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}

#[must_use]
pub fn tracked_calls(_config: &Config) -> HashSet<String> {
    HashSet::new()
}
