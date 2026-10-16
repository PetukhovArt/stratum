use std::collections::BTreeSet;

use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    types::ContainerKind,
    violation::Violation,
};
use stratum_graph::{CompoundGraph, DirRole};

use super::folder::at_folder;
use crate::ids::STAGE_MIXING;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct StageMixing;

impl Rule for StageMixing {
    type Scope = ContainerId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        STAGE_MIXING
    }
    fn slug(&self) -> &'static str {
        "stratum/stage-mixing"
    }
    fn default_severity(&self) -> Severity {
        Severity::Off
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ContainerId,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(c) = graph
            .containers
            .get(&scope)
            .filter(|c| c.kind != ContainerKind::Layer)
        else {
            return vec![];
        };
        let segments: BTreeSet<&str> = c.segment_dirs.iter().map(String::as_str).collect();
        if segments.is_empty() {
            return vec![];
        }
        let sub_units: BTreeSet<&str> = graph
            .containers
            .values()
            .filter(|s| s.parent == Some(scope) && s.kind == ContainerKind::SubUnit)
            .map(|s| s.name.as_str())
            .collect();
        let domain_folders: BTreeSet<&str> = graph
            .dir_entries(&c.path)
            .into_iter()
            .flat_map(|entries| &entries.dirs)
            .filter(|name| graph.dir_roles.get(&c.path.join(name)) == Some(&DirRole::Collection))
            .map(String::as_str)
            .collect();
        if sub_units.is_empty() && domain_folders.is_empty() {
            return vec![];
        }
        let list = |names: &BTreeSet<&str>| names.iter().copied().collect::<Vec<_>>().join(", ");
        let mut others = Vec::new();
        if !sub_units.is_empty() {
            others.push(format!("sub-units ({})", list(&sub_units)));
        }
        if !domain_folders.is_empty() {
            others.push(format!("other folders ({})", list(&domain_folders)));
        }
        let fix = if sub_units.is_empty() {
            format!(
                "Move the code of {} into the segment folders ({}), or turn the segments into sub-units: not both on one level.",
                list(&domain_folders),
                list(&segments),
            )
        } else {
            "Pick one split: move the segment folders' code into sub-units, or fold the sub-units back into segments.".into()
        };
        at_folder(
            graph,
            self.id(),
            severity,
            &c.path,
            c.entry,
            format!(
                "Container '{name}' mixes segment folders ({segments}) with {others} on one level",
                name = c.name,
                segments = list(&segments),
                others = others.join(" and "),
            ),
            fix,
        )
        .into_iter()
        .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use stratum_core::{
        ids::{LayerId, ModuleId},
        purity::Purity,
        stage::Stage,
        types::{Container, Module},
        visibility::VisibilityScope,
    };

    fn container(id: u32, path: &str, kind: ContainerKind, segments: &[&str]) -> Container {
        Container {
            id: ContainerId::new(id),
            name: path.rsplit('/').next().unwrap().into(),
            path: PathBuf::from(path),
            kind,
            layer: LayerId::new(0),
            parent: id.checked_sub(1).map(ContainerId::new),
            stage: Stage::FLAT,
            entry: None,
            segment_dirs: segments.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn check(segments: &[&str], sub_unit: Option<&[&str]>) -> Vec<Violation> {
        let mut g = CompoundGraph::empty();
        let mut cs = vec![
            container(0, "src/features", ContainerKind::Layer, &[]),
            container(1, "src/features/cart", ContainerKind::Slice, segments),
        ];
        if let Some(inner) = sub_unit {
            cs.push(container(
                2,
                "src/features/cart/checkout",
                ContainerKind::SubUnit,
                inner,
            ));
        }
        for c in cs {
            g.containers.insert(c.id, c);
        }
        let id = ModuleId::new(0);
        g.modules.insert(
            id,
            Module {
                id,
                path: PathBuf::from("src/features/cart/index.ts"),
                container: ContainerId::new(1),
                layer: LayerId::new(0),
                purity: Purity::new(2).unwrap(),
                visibility: VisibilityScope::Public,
                exports: stratum_core::types::Exports::default(),
                calls: vec![],
                imports: vec![],
                suppressions: vec![],
            },
        );
        StageMixing.check(&g, ContainerId::new(1), &EmptyOptions, Severity::Error)
    }

    #[test]
    fn segment_next_to_sub_unit_is_mixing() {
        let v = check(&["model"], Some(&[]));
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("(model)") && v[0].message.contains("(checkout)"));
    }

    #[test]
    fn sub_units_or_segments_alone_are_not_mixing() {
        assert!(check(&[], Some(&[])).is_empty());
        assert!(check(&["model"], None).is_empty());
        assert!(check(&[], Some(&["model"])).is_empty());
    }
}
