use std::collections::BTreeSet;

use stratum_core::{
    ids::{ContainerId, RuleId},
    severity::Severity,
    types::{Container, ContainerKind},
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::SLICE_FAN_IN;
use crate::options::SliceFanInOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct SliceFanIn;

impl Rule for SliceFanIn {
    type Scope = ContainerId;
    type Options = SliceFanInOptions;

    fn id(&self) -> RuleId {
        SLICE_FAN_IN
    }
    fn slug(&self) -> &'static str {
        "stratum/slice-fan-in"
    }
    fn default_severity(&self) -> Severity {
        Severity::Off
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ContainerId,
        options: &SliceFanInOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(slice) = graph
            .containers
            .get(&scope)
            .filter(|c| c.kind == ContainerKind::Slice)
        else {
            return vec![];
        };
        let importers: BTreeSet<String> = graph
            .importing_slices(scope)
            .iter()
            .filter_map(|c| graph.containers.get(c))
            .map(|c| qualified(graph, c))
            .collect();
        let fan_in = u32::try_from(importers.len()).unwrap_or(u32::MAX);
        if fan_in <= options.max_fan_in {
            return vec![];
        }
        let first_member = || {
            graph
                .modules
                .keys()
                .copied()
                .find(|&m| graph.outer_container(m).is_some_and(|c| c.id == scope))
        };
        let Some(rep) = slice.entry.or_else(first_member) else {
            return vec![];
        };
        let Some(file) = graph.modules.get(&rep).map(|m| m.path.clone()) else {
            return vec![];
        };
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Slice {slice} is imported by {fan_in} other slices (limit {limit}): {list}",
                slice = qualified(graph, slice),
                limit = options.max_fan_in,
                list = importers.into_iter().collect::<Vec<_>>().join(", "),
            ),
            file,
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![rep],
            edge: None,
            fix: Some(
                "Narrow the slice's entry or split it so each importer reaches only the part it needs; code every slice needs may belong in a lower layer.".into(),
            ),
        }]
    }
}

fn qualified(graph: &CompoundGraph, slice: &Container) -> String {
    let layer = graph
        .layers
        .get(&slice.layer)
        .map_or("?", |l| l.name.as_str());
    format!("{layer}/{}", slice.name)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use stratum_core::{
        edge::EdgeKind,
        ids::{LayerId, ModuleId},
        purity::Purity,
        stage::Stage,
        types::Module,
        visibility::VisibilityScope,
    };

    fn build(slices: u32, modules: &[u32], edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for id in 0..=slices {
            let (kind, parent, path) = if id == 0 {
                (ContainerKind::Layer, None, "src/l".to_owned())
            } else {
                (
                    ContainerKind::Slice,
                    Some(ContainerId::new(0)),
                    format!("src/l/s{id}"),
                )
            };
            let id = ContainerId::new(id);
            g.containers.insert(
                id,
                Container {
                    id,
                    name: path.rsplit('/').next().unwrap().into(),
                    path: PathBuf::from(path),
                    kind,
                    layer: LayerId::new(0),
                    parent,
                    stage: Stage::FLAT,
                    entry: None,
                    segment_dirs: vec![],
                },
            );
        }
        for (i, slice) in modules.iter().enumerate() {
            let id = ModuleId::new(u32::try_from(i).unwrap());
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
            g.modules.insert(
                id,
                Module {
                    id,
                    path: PathBuf::from(format!("src/l/s{slice}/m{i}.ts")),
                    container: ContainerId::new(*slice),
                    layer: LayerId::new(0),
                    purity: Purity::new(2).unwrap(),
                    visibility: VisibilityScope::Public,
                    exports: stratum_core::types::Exports::default(),
                    calls: vec![],
                    imports: vec![],
                    suppressions: vec![],
                },
            );
        }
        for (a, b) in edges {
            let na = g.node_index[&ModuleId::new(*a)];
            let nb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    fn check(g: &CompoundGraph) -> Vec<Violation> {
        SliceFanIn.check(
            g,
            ContainerId::new(1),
            &SliceFanInOptions { max_fan_in: 2 },
            Severity::Warning,
        )
    }

    #[test]
    fn many_modules_of_one_slice_count_once() {
        let g = build(
            3,
            &[1, 1, 2, 2, 2, 2, 3],
            &[(2, 0), (3, 1), (4, 0), (5, 1), (6, 0)],
        );
        assert!(check(&g).is_empty());
    }

    #[test]
    fn distinct_importing_slices_over_the_limit_are_reported() {
        let g = build(4, &[1, 1, 2, 3, 4], &[(2, 0), (3, 1), (4, 0), (1, 0)]);
        let v = check(&g);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("3 other slices"));
    }
}
