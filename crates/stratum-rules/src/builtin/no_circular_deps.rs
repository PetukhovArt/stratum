use std::path::Path;

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{Breaks, CompoundGraph, Cut, Cycle, break_candidates, find_cycles};

use crate::ids::NO_CIRCULAR_DEPS;
use crate::rule::{EmptyOptions, Rule};
use crate::scope::ProjectScope;

const ANCHOR_NAME_CHARS: usize = 120;

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
        let mut out: Vec<Violation> = find_cycles(graph)
            .iter()
            .map(|cycle| self.finding(graph, cycle, severity))
            .collect();
        out.sort_by(|a, b| a.file.cmp(&b.file));
        out
    }
}

impl NoCircularDeps {
    fn finding(self, graph: &CompoundGraph, cycle: &Cycle, severity: Severity) -> Violation {
        let members = cycle.by_path(graph);
        let anchor = members.first().copied();
        let Breaks { cuts, closing } = break_candidates(graph, cycle);

        let name = anchor
            .and_then(|m| path_of(graph, m).file_name())
            .map(|n| {
                n.to_string_lossy()
                    .chars()
                    .take(ANCHOR_NAME_CHARS)
                    .collect::<String>()
            })
            .unwrap_or_default();
        let message = format!(
            "Import cycle of {} modules through {name}; about {} imports close it",
            members.len(),
            closing,
        );
        let fix = format!(
            "Break it at one of: {}; extract the shared piece into a lower module or invert the dependency",
            cuts.iter()
                .map(|cut| describe(graph, cut, members.len()))
                .collect::<Vec<_>>()
                .join(", "),
        );

        let mut modules: Vec<ModuleId> = Vec::with_capacity(members.len());
        for cut in &cuts {
            for m in [cut.from, cut.to] {
                if !modules.contains(&m) {
                    modules.push(m);
                }
            }
        }
        let rest: Vec<ModuleId> = members
            .iter()
            .filter(|m| !modules.contains(m))
            .copied()
            .collect();
        modules.extend(rest);

        Violation {
            rule: self.id(),
            severity,
            message,
            file: anchor
                .map(|m| path_of(graph, m).to_path_buf())
                .unwrap_or_default(),
            location: SourceLocation {
                line: anchor.map_or(1, |m| first_import_line(graph, m, &members)),
                column: 1,
            },
            modules,
            edge: None,
            suggestion: Some(fix),
        }
    }
}

fn path_of(graph: &CompoundGraph, m: ModuleId) -> &Path {
    graph
        .modules
        .get(&m)
        .map_or(Path::new(""), |module| module.path.as_path())
}

fn display(graph: &CompoundGraph, m: ModuleId) -> String {
    path_of(graph, m).to_string_lossy().replace('\\', "/")
}

fn describe(graph: &CompoundGraph, cut: &Cut, size: usize) -> String {
    let at = import_line(graph, cut.from, cut.to)
        .map(|line| format!(":{line}"))
        .unwrap_or_default();
    format!(
        "`{}{at} -> {}` (frees {} of {size})",
        display(graph, cut.from),
        display(graph, cut.to),
        cut.freed,
    )
}

fn first_import_line(graph: &CompoundGraph, anchor: ModuleId, members: &[ModuleId]) -> u32 {
    let Some(node) = graph.node_for(anchor) else {
        return 1;
    };
    graph
        .deps
        .neighbors(node)
        .map(|next| graph.deps[next])
        .filter(|to| members.contains(to))
        .filter_map(|to| import_line(graph, anchor, to))
        .min()
        .unwrap_or(1)
}

// TODO: once `CompoundGraph::import_location(from, to)` (slice A1) is on main,
// return its line here; until then findings sit on line 1 and cuts carry no line.
fn import_line(_graph: &CompoundGraph, _from: ModuleId, _to: ModuleId) -> Option<u32> {
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId, ModuleId},
        purity::Purity,
        types::Module,
        visibility::VisibilityScope,
    };

    fn build(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for i in 0..n {
            let id = ModuleId::new(i);
            let m = Module {
                id,
                path: PathBuf::from(format!("src/m{i}.ts")),
                container: ContainerId::new(0),
                layer: LayerId::new(0),
                purity: Purity::new(2).unwrap(),
                visibility: VisibilityScope::Public,
                exports: stratum_core::types::Exports::default(),
                calls: vec![],
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

    fn check(g: &CompoundGraph) -> Vec<Violation> {
        NoCircularDeps.check(g, ProjectScope, &EmptyOptions, Severity::Error)
    }

    #[test]
    fn two_node_cycle_emits_one_violation() {
        let g = build(2, &[(0, 1), (1, 0)]);
        let viols = check(&g);
        assert_eq!(viols.len(), 1);
        assert_eq!(viols[0].rule, NO_CIRCULAR_DEPS);
        assert_eq!(viols[0].modules.len(), 2);
    }

    #[test]
    fn finding_sits_on_the_smallest_path_and_lists_cuts_first() {
        let g = build(4, &[(3, 2), (2, 1), (1, 0), (0, 3), (1, 3)]);
        let v = &check(&g)[0];

        assert_eq!(v.file, PathBuf::from("src/m0.ts"));
        assert_eq!(
            v.message,
            "Import cycle of 4 modules through m0.ts; about 1 imports close it"
        );
        let fix = v.suggestion.as_deref().unwrap();
        assert!(fix.starts_with("Break it at one of: `src/m"), "{fix}");
        assert!(fix.contains("(frees 4 of 4)"), "{fix}");
        assert_eq!(v.modules.len(), 4);
        assert_eq!(v.modules[..2], [ModuleId::new(2), ModuleId::new(1)]);
        assert_eq!(fix.matches(" -> ").count(), 3);
    }

    #[test]
    fn the_fingerprint_survives_a_change_of_candidates() {
        let ring = build(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]);
        let with_chord = build(4, &[(0, 1), (1, 2), (2, 3), (3, 0), (2, 0)]);
        let identity = |g: &CompoundGraph| {
            let v = check(g).remove(0);
            let masked: String = v.message.chars().filter(|c| !c.is_ascii_digit()).collect();
            (v.file, masked)
        };

        assert_ne!(
            check(&ring)[0].suggestion,
            check(&with_chord)[0].suggestion,
            "the chord must change the candidates"
        );
        assert_eq!(identity(&ring), identity(&with_chord));
    }
}
