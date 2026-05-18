use stratum_core::{
    ids::RuleId,
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, find_cycles};

use crate::ids::NO_CIRCULAR_DEPS;
use crate::rule::{EmptyOptions, Rule};
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoCircularDeps;

impl Rule for NoCircularDeps {
    type Scope = ProjectScope;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        NO_CIRCULAR_DEPS
    }
    fn slug(&self) -> &'static str {
        "stratum/no-circular-deps"
    }
    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let mut out = Vec::new();
        for cycle in find_cycles(graph) {
            let Some(&first_id) = cycle.modules.first() else {
                continue;
            };
            let file = graph
                .modules
                .get(&first_id)
                .map(|m| m.path.clone())
                .unwrap_or_default();
            let names: Vec<String> = cycle
                .modules
                .iter()
                .map(|m| {
                    graph
                        .modules
                        .get(m)
                        .and_then(|mm| mm.path.file_name())
                        .map_or_else(
                            || format!("module#{}", m.raw()),
                            |s| s.to_string_lossy().to_string(),
                        )
                })
                .collect();
            out.push(Violation {
                rule: self.id(),
                severity,
                message: format!("Circular dependency: {}", names.join(" -> ")),
                file,
                location: SourceLocation { line: 1, column: 1 },
                modules: cycle.modules.clone(),
                edge: None,
                suggestion: Some(
                    "Break the cycle by extracting the shared piece into a lower-stage module or by inverting one of the dependencies."
                        .into(),
                ),
            });
        }
        out
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
        stage::Stage,
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
                stage: Stage::new(2).unwrap(),
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
    fn dag_emits_no_violation() {
        let g = build(3, &[(0, 1), (1, 2)]);
        let viols = NoCircularDeps.check(&g, ProjectScope, &EmptyOptions, Severity::Error);
        assert!(viols.is_empty());
    }

    #[test]
    fn two_node_cycle_emits_one_violation() {
        let g = build(2, &[(0, 1), (1, 0)]);
        let viols = NoCircularDeps.check(&g, ProjectScope, &EmptyOptions, Severity::Error);
        assert_eq!(viols.len(), 1);
        assert_eq!(viols[0].rule, NO_CIRCULAR_DEPS);
        assert_eq!(viols[0].modules.len(), 2);
    }

    #[test]
    fn two_disjoint_cycles_emit_two_violations() {
        let g = build(4, &[(0, 1), (1, 0), (2, 3), (3, 2)]);
        let viols = NoCircularDeps.check(&g, ProjectScope, &EmptyOptions, Severity::Error);
        assert_eq!(viols.len(), 2);
    }
}
