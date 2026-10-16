use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::ids::COMPOSER_SCOPE;
use crate::options::ComposerScopeOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct ComposerScope;

impl Rule for ComposerScope {
    type Scope = ContainerId;
    type Options = ComposerScopeOptions;

    fn id(&self) -> RuleId {
        COMPOSER_SCOPE
    }
    fn slug(&self) -> &'static str {
        "stratum/composer-scope"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ContainerId,
        _options: &ComposerScopeOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}
