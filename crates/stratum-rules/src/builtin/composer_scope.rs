use globset::Glob;
use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    stage::Stage,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use super::folder::{at_folder, private_zone_phrase};
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
        graph: &CompoundGraph,
        scope: ContainerId,
        options: &ComposerScopeOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(pattern) = options.scope_file.as_deref() else {
            return vec![];
        };
        let Some(c) = graph
            .containers
            .get(&scope)
            .filter(|c| c.stage == Stage::COMPOSED)
        else {
            return vec![];
        };
        let Ok(glob) = Glob::new(pattern).map(|g| g.compile_matcher()) else {
            return vec![];
        };
        let has_scope_file = graph
            .private_zones_in(scope)
            .flat_map(|zone| &zone.files)
            .filter_map(|m| graph.modules.get(m)?.path.file_name())
            .any(|name| glob.is_match(name));
        if has_scope_file {
            return vec![];
        }
        let zone = private_zone_phrase(graph);
        at_folder(
            graph,
            COMPOSER_SCOPE,
            severity,
            &c.path,
            c.entry,
            format!(
                "Composer '{name}' has no file matching '{pattern}' in its private zone",
                name = c.name,
            ),
            format!(
                "Add a file matching '{pattern}' to {zone} inside '{name}' and move the wiring of its sub-units into it.",
                name = c.name,
            ),
        )
        .into_iter()
        .collect()
    }
}
