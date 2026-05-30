use stratum_core::{
    ids::RuleId,
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, find_cycles};

use crate::ids::NO_CIRCULAR_DEPS;
use crate::rule::{EmptyOptions, Rule};
use crate::scope::ProjectScope;

/// Above this many modules, a cycle's message folds its tail instead of listing
/// every member inline.
const MAX_INLINE: usize = 8;

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
            // A 953-module SCC dumped as `a -> b -> c -> …` on one line is
            // unreadable. Lead with the module count and fold the tail.
            let total = cycle.modules.len();
            let message = if total > MAX_INLINE {
                let head = names
                    .iter()
                    .take(MAX_INLINE)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" -> ");
                format!(
                    "Circular dependency across {total} modules: {head} -> … (+{} more)",
                    total - MAX_INLINE
                )
            } else {
                format!("Circular dependency: {}", names.join(" -> "))
            };
            out.push(Violation {
                rule: self.id(),
                severity,
                message,
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

    #[test]
    fn small_cycle_lists_every_module_inline() {
        let g = build(3, &[(0, 1), (1, 2), (2, 0)]);
        let viols = NoCircularDeps.check(&g, ProjectScope, &EmptyOptions, Severity::Error);
        assert_eq!(viols.len(), 1);
        let msg = &viols[0].message;
        assert!(msg.starts_with("Circular dependency: "));
        assert!(msg.contains("m0.ts -> m1.ts -> m2.ts"));
        assert!(!msg.contains("more"));
    }

    #[test]
    fn fold_boundary_is_strictly_above_eight() {
        // total == 8: still inline (no fold), pins `>` not `>=`.
        let ring8: Vec<(u32, u32)> = (0..8).map(|i| (i, (i + 1) % 8)).collect();
        let g8 = build(8, &ring8);
        let m8 =
            &NoCircularDeps.check(&g8, ProjectScope, &EmptyOptions, Severity::Error)[0].message;
        assert!(m8.starts_with("Circular dependency: "), "got: {m8}");
        assert!(!m8.contains("more"), "8 must not fold: {m8}");

        // total == 9: folds with exactly "(+1 more)".
        let ring9: Vec<(u32, u32)> = (0..9).map(|i| (i, (i + 1) % 9)).collect();
        let g9 = build(9, &ring9);
        let m9 =
            &NoCircularDeps.check(&g9, ProjectScope, &EmptyOptions, Severity::Error)[0].message;
        assert!(m9.contains("across 9 modules"), "got: {m9}");
        assert!(m9.contains("(+1 more)"), "got: {m9}");
    }

    #[test]
    fn large_cycle_is_folded_with_count_and_tail_summary() {
        // 12-module ring: 0->1->...->11->0.
        let mut edges: Vec<(u32, u32)> = (0..11).map(|i| (i, i + 1)).collect();
        edges.push((11, 0));
        let g = build(12, &edges);
        let viols = NoCircularDeps.check(&g, ProjectScope, &EmptyOptions, Severity::Error);
        assert_eq!(viols.len(), 1);
        let msg = &viols[0].message;
        // count-first, folded: 8 shown, 4 hidden.
        assert!(msg.contains("across 12 modules"), "got: {msg}");
        assert!(msg.contains("(+4 more)"), "got: {msg}");
        // the folded tail is NOT dumped inline
        assert!(!msg.contains("m11.ts"), "tail leaked: {msg}");
        // the full membership is still available programmatically
        assert_eq!(viols[0].modules.len(), 12);
    }
}
