use std::collections::BTreeSet;

use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    types::ContainerKind,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::STAGE_MIXING;
use crate::options::StageMixingOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct StageMixing;

impl Rule for StageMixing {
    type Scope = ContainerId;
    type Options = StageMixingOptions;

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
        options: &StageMixingOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(c) = graph.containers.get(&scope) else {
            return vec![];
        };
        let sub_units: BTreeSet<&str> = graph
            .containers
            .values()
            .filter(|s| s.parent == Some(scope) && s.kind == ContainerKind::SubUnit)
            .map(|s| s.name.as_str())
            .collect();
        if sub_units.is_empty() {
            return vec![];
        }
        let segments: BTreeSet<&str> = graph
            .modules
            .values()
            .filter_map(|m| {
                let rest = m.path.strip_prefix(&c.path).ok()?;
                let mut parts = rest.iter();
                let first = parts.next()?.to_str()?;
                parts.next()?;
                options.segments.iter().any(|s| s == first).then_some(first)
            })
            .collect();
        if segments.is_empty() {
            return vec![];
        }
        let Some(rep) = c.entry.or_else(|| {
            graph
                .modules
                .values()
                .find(|m| m.container == scope)
                .map(|m| m.id)
        }) else {
            return vec![];
        };
        let Some(file) = graph.modules.get(&rep).map(|m| m.path.clone()) else {
            return vec![];
        };
        let list = |names: &BTreeSet<&str>| names.iter().copied().collect::<Vec<_>>().join(", ");
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Container '{name}' mixes segment folders ({segments}) with sub-units ({sub_units}) on one level",
                name = c.name,
                segments = list(&segments),
                sub_units = list(&sub_units),
            ),
            file,
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![rep],
            edge: None,
            suggestion: Some(
                "Pick one split: move the segment folders' code into sub-units, or fold the sub-units back into segments.".into(),
            ),
        }]
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

    fn container(id: u32, path: &str, kind: ContainerKind, parent: Option<u32>) -> Container {
        Container {
            id: ContainerId::new(id),
            name: path.rsplit('/').next().unwrap().into(),
            path: PathBuf::from(path),
            kind,
            layer: LayerId::new(0),
            parent: parent.map(ContainerId::new),
            stage: Stage::FLAT,
            entry: None,
        }
    }

    fn graph(files: &[(&str, u32)], sub_unit: Option<&str>) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        let mut cs = vec![
            container(0, "src/features", ContainerKind::Layer, None),
            container(1, "src/features/cart", ContainerKind::Slice, Some(0)),
        ];
        if let Some(path) = sub_unit {
            cs.push(container(2, path, ContainerKind::SubUnit, Some(1)));
        }
        for c in cs {
            g.containers.insert(c.id, c);
        }
        for (i, (path, container)) in files.iter().enumerate() {
            let id = ModuleId::new(u32::try_from(i).unwrap());
            g.modules.insert(
                id,
                Module {
                    id,
                    path: PathBuf::from(path),
                    container: ContainerId::new(*container),
                    layer: LayerId::new(0),
                    purity: Purity::new(2).unwrap(),
                    visibility: VisibilityScope::Public,
                    exports: stratum_core::types::Exports::default(),
                    calls: vec![],
                },
            );
        }
        g
    }

    fn check(g: &CompoundGraph) -> Vec<Violation> {
        let options = StageMixingOptions {
            segments: vec!["model".into(), "ui".into()],
        };
        StageMixing.check(g, ContainerId::new(1), &options, Severity::Error)
    }

    #[test]
    fn segment_next_to_sub_unit_is_mixing() {
        let g = graph(
            &[
                ("src/features/cart/model/store.ts", 1),
                ("src/features/cart/checkout/index.ts", 2),
            ],
            Some("src/features/cart/checkout"),
        );
        let v = check(&g);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("(model)") && v[0].message.contains("(checkout)"));
    }

    #[test]
    fn collection_folder_or_segments_alone_are_not_mixing() {
        let collection = graph(
            &[
                ("src/features/cart/helpers/a.ts", 1),
                ("src/features/cart/checkout/index.ts", 2),
            ],
            Some("src/features/cart/checkout"),
        );
        let segments_only = graph(&[("src/features/cart/model/store.ts", 1)], None);
        let segment_in_sub_unit = graph(
            &[("src/features/cart/checkout/model/a.ts", 2)],
            Some("src/features/cart/checkout"),
        );
        assert!(check(&collection).is_empty());
        assert!(check(&segments_only).is_empty());
        assert!(check(&segment_in_sub_unit).is_empty());
    }
}
