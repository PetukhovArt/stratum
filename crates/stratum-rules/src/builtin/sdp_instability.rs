use stratum_core::{
    edge::Edge,
    ids::RuleId,
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, metrics::sdp_violations};

use crate::ids::INSTABILITY;
use crate::rule::{EmptyOptions, Rule};
use crate::scope::ProjectScope;

/// Flags edges that violate the Stable Dependencies Principle: a module depends
/// on another whose instability `I = Ce/(Ca+Ce)` is *higher* (less stable).
///
/// Advisory by default — the raw metric fires broadly until calibrated (T14).
#[derive(Debug, Default, Clone, Copy)]
pub struct Instability;

impl Rule for Instability {
    type Scope = ProjectScope;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        INSTABILITY
    }
    fn slug(&self) -> &'static str {
        "stratum/instability"
    }
    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let name = |id: stratum_core::ids::ModuleId| {
            graph.modules.get(&id).map_or_else(
                || format!("module#{}", id.raw()),
                |m| {
                    m.path.file_name().map_or_else(
                        || m.path.to_string_lossy().to_string(),
                        |s| s.to_string_lossy().to_string(),
                    )
                },
            )
        };
        sdp_violations(graph)
            .into_iter()
            .map(|v| {
                let kind = graph
                    .node_for(v.from)
                    .zip(graph.node_for(v.to))
                    .and_then(|(a, b)| graph.deps.find_edge(a, b))
                    .map(|e| graph.deps[e]);
                let file = graph.modules.get(&v.from).map(|m| m.path.clone()).unwrap_or_default();
                Violation {
                    rule: self.id(),
                    severity,
                    message: format!(
                        "{} (I={:.2}) depends on {} (I={:.2}): dependencies should point toward more stable code (Stable Dependencies Principle)",
                        name(v.from),
                        v.from_instability,
                        name(v.to),
                        v.to_instability
                    ),
                    file,
                    location: SourceLocation { line: 1, column: 1 },
                    modules: vec![v.from, v.to],
                    edge: kind.map(|kind| Edge {
                        from: v.from,
                        to: v.to,
                        kind,
                    }),
                    suggestion: Some(
                        "Invert the dependency (depend on an abstraction) or make the target more stable."
                            .into(),
                    ),
                }
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId, ModuleId},
        purity::Purity,
        types::Module,
        visibility::VisibilityScope,
    };

    fn build(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
        for i in 0..n {
            let id = ModuleId::new(i);
            let m = Module {
                id,
                path: PathBuf::from(format!("src/m{i}.ts")),
                container: ContainerId::new(0),
                layer: LayerId::new(0),
                purity: Purity::new(2).unwrap(),
                visibility: VisibilityScope::Public,
            };
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
            g.modules.insert(id, m);
        }
        for (a, b) in edges {
            let na = g.node_index[&ModuleId::new(*a)];
            let nb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn well_ordered_graph_emits_nothing() {
        let g = build(3, &[(0, 1), (1, 2)]);
        let viols = Instability.check(&g, ProjectScope, &EmptyOptions, Severity::Info);
        assert!(viols.is_empty());
    }

    #[test]
    fn flags_stable_hub_depending_on_unstable_leaf() {
        // hub 0 (I=0.25) depends on leaf 3 (I=0.5).
        let g = build(4, &[(1, 0), (2, 0), (3, 0), (0, 3)]);
        let viols = Instability.check(&g, ProjectScope, &EmptyOptions, Severity::Info);
        assert_eq!(viols.len(), 1);
        assert_eq!(viols[0].rule, INSTABILITY);
        assert_eq!(viols[0].modules, vec![ModuleId::new(0), ModuleId::new(3)]);
        assert!(
            viols[0].edge.is_some(),
            "edge should be attached for the lens"
        );
        assert!(viols[0].message.contains("Stable Dependencies Principle"));
    }
}
