use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    types::{Container, ContainerKind},
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::builtin::public_api::{Finding, edge_violations, rel_path};
use crate::ids::SIBLING_ISOLATION;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct SiblingIsolation;

impl Rule for SiblingIsolation {
    type Scope = ModuleId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        SIBLING_ISOLATION
    }
    fn slug(&self) -> &'static str {
        "stratum/sibling-isolation"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let from_chain: Vec<&Container> = graph.containers_of(scope).collect();
        edge_violations(graph, scope, self.id(), severity, |to| {
            let to_chain: Vec<&Container> = graph.containers_of(to).collect();
            let (from_unit, to_unit, parent) = siblings(&from_chain, &to_chain)?;
            let isolated =
                |c: &Container| matches!(c.kind, ContainerKind::Slice | ContainerKind::SubUnit);
            if !isolated(from_unit) || !isolated(to_unit) {
                return None;
            }
            let rel = |c: &Container| rel_path(graph, c.layer, &c.path);
            let (from_unit, to_unit, parent) = (rel(from_unit), rel(to_unit), rel(parent));
            Some(Finding {
                message: format!(
                    "`{from_unit}` imports its sibling `{to_unit}`; siblings inside `{parent}` must stay independent"
                ),
                suggestion: format!(
                    "Move the shared code down (a lower layer or a private zone of `{parent}`), or compose `{from_unit}` and `{to_unit}` in `{parent}`."
                ),
            })
        })
    }
}

fn siblings<'g>(
    from: &[&'g Container],
    to: &[&'g Container],
) -> Option<(&'g Container, &'g Container, &'g Container)> {
    let (fi, ti) = from
        .iter()
        .enumerate()
        .find_map(|(fi, c)| to.iter().position(|t| t.id == c.id).map(|ti| (fi, ti)))?;
    Some((
        from.get(fi.checked_sub(1)?)?,
        to.get(ti.checked_sub(1)?)?,
        from[fi],
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::builtin::public_api::fixture::Fixture;
    use stratum_core::ids::ContainerId;

    fn check(f: &Fixture, from: ModuleId) -> Vec<Violation> {
        SiblingIsolation.check(&f.g, from, &EmptyOptions, Severity::Error)
    }

    fn slices() -> (Fixture, ContainerId, ContainerId) {
        let mut f = Fixture::default();
        let layer = f.layer(0, "src/features");
        let cart = f.unit("src/features/cart", ContainerKind::Slice, layer);
        let user = f.unit("src/features/user", ContainerKind::Slice, layer);
        (f, cart, user)
    }

    #[test]
    fn slices_of_one_layer_are_isolated() {
        let (mut f, cart, user) = slices();
        let a = f.entry(cart, "src/features/cart/index.ts");
        let b = f.entry(user, "src/features/user/index.ts");
        f.edge(a, b);
        let v = check(&f, a);
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "`src/features/cart` imports its sibling `src/features/user`; siblings inside `src/features` must stay independent"
        );
    }

    #[test]
    fn sub_units_at_depth_two_are_isolated() {
        let (mut f, _, user) = slices();
        let profile = f.unit("src/features/user/profile", ContainerKind::SubUnit, user);
        let auth = f.unit("src/features/user/auth", ContainerKind::SubUnit, user);
        f.unit("src/features/user/auth/otp", ContainerKind::SubUnit, auth);
        let token = f.unit("src/features/user/auth/token", ContainerKind::SubUnit, auth);
        let form = f.module("src/features/user/auth/otp/ui/form.vue", None);
        let token_entry = f.entry(token, "src/features/user/auth/token/index.ts");
        let profile_entry = f.entry(profile, "src/features/user/profile/index.ts");
        f.edge(form, token_entry);
        f.edge(form, profile_entry);
        let v = check(&f, form);
        assert_eq!(v.len(), 2);
        assert!(v[0].message.starts_with(
            "`src/features/user/auth/otp` imports its sibling `src/features/user/auth/token`"
        ));
        assert!(v[1].message.starts_with(
            "`src/features/user/auth` imports its sibling `src/features/user/profile`"
        ));
    }

    #[test]
    fn parent_importing_its_child_is_not_a_sibling_edge() {
        let (mut f, _, user) = slices();
        let profile = f.unit("src/features/user/profile", ContainerKind::SubUnit, user);
        let a = f.module("src/features/user/ui/page.vue", None);
        let b = f.entry(profile, "src/features/user/profile/index.ts");
        f.edge(a, b);
        f.edge(b, a);
        assert!(check(&f, a).is_empty());
        assert!(check(&f, b).is_empty());
    }

    #[test]
    fn nested_containers_are_not_isolated() {
        let (mut f, cart, _) = slices();
        let button = f.unit("src/features/cart/ui/button", ContainerKind::Nested, cart);
        let badge = f.unit("src/features/cart/ui/badge", ContainerKind::Nested, cart);
        let a = f.entry(button, "src/features/cart/ui/button/index.ts");
        let b = f.entry(badge, "src/features/cart/ui/badge/index.ts");
        f.edge(a, b);
        assert!(check(&f, a).is_empty());
    }

    #[test]
    fn entry_bearing_children_of_an_unsliced_layer_are_not_isolated() {
        let mut f = Fixture::default();
        let kit = f.layer(0, "src/shared");
        let button = f.unit("src/shared/button", ContainerKind::Nested, kit);
        let icon = f.unit("src/shared/icon", ContainerKind::Nested, kit);
        let a = f.entry(button, "src/shared/button/index.ts");
        let b = f.entry(icon, "src/shared/icon/index.ts");
        f.edge(a, b);
        assert!(check(&f, a).is_empty());
    }
}
