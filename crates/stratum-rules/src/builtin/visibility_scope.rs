use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::Violation,
    visibility::VisibilityScope as VS,
};
use stratum_graph::CompoundGraph;

use crate::builtin::edge::{EdgeJudge, Finding, edge_violations};
use crate::ids::VISIBILITY_SCOPE;
use crate::paths::project_relative;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct VisibilityScope;

impl Rule for VisibilityScope {
    type Scope = ModuleId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        VISIBILITY_SCOPE
    }
    fn slug(&self) -> &'static str {
        "stratum/visibility-scope"
    }
    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn edge_judge(&self) -> Option<EdgeJudge<EmptyOptions>> {
        Some(EdgeJudge(|graph, from, to, _| judge(graph, from, to)))
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        edge_violations(graph, scope, self.id(), severity, |to| {
            judge(graph, scope, to)
        })
    }
}

pub(crate) fn judge(graph: &CompoundGraph, from: ModuleId, to: ModuleId) -> Option<Finding> {
    let from = graph.modules.get(&from)?;
    let target = graph.modules.get(&to)?;
    let VS::Private { root } = &target.visibility else {
        return None;
    };
    if from.path.starts_with(root) {
        return None;
    }
    let root = project_relative(graph, target.layer, root);
    let target = project_relative(graph, target.layer, &target.path);
    Some(Finding {
        message: format!(
            "`{target}` is in a private zone of `{root}`; only modules under `{root}` may import it"
        ),
        suggestion: Some(format!(
            "Import it through the entry of the container that owns `{root}`, move the importer under `{root}`, or move the code out of the private zone if it is meant to be shared."
        )),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::builtin::public_api::fixture::Fixture;
    use stratum_core::types::ContainerKind;

    fn check(f: &Fixture, from: ModuleId) -> Vec<Violation> {
        VisibilityScope.check(&f.g, from, &EmptyOptions, Severity::Error)
    }

    fn zone() -> (Fixture, ModuleId) {
        let mut f = Fixture::default();
        let layer = f.layer(0, "src/entities");
        f.unit("src/entities/user", ContainerKind::Slice, layer);
        f.unit("src/entities/user2", ContainerKind::Slice, layer);
        let helper = f.module(
            "src/entities/user/_shared/helper.ts",
            Some("src/entities/user"),
        );
        (f, helper)
    }

    #[test]
    fn private_zone_is_visible_to_its_root_subtree() {
        let (mut f, helper) = zone();
        let a = f.module("src/entities/user/profile/ui/card.vue", None);
        f.edge(a, helper);
        assert!(check(&f, a).is_empty());
    }

    #[test]
    fn cousin_path_sharing_the_folder_name_prefix_is_rejected() {
        let (mut f, helper) = zone();
        let a = f.module(
            "src/entities/user2/_shared/helper.ts",
            Some("src/entities/user2"),
        );
        f.edge(a, helper);
        let v = check(&f, a);
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "`src/entities/user/_shared/helper.ts` is in a private zone of `src/entities/user`; only modules under `src/entities/user` may import it"
        );
    }

    #[test]
    fn public_target_is_allowed() {
        let (mut f, _) = zone();
        let a = f.module("src/entities/user2/a.ts", None);
        let b = f.module("src/entities/user/b.ts", None);
        f.edge(a, b);
        assert!(check(&f, a).is_empty());
    }
}
