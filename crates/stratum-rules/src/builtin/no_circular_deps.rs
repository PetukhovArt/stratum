use std::path::Path;

use rustc_hash::FxHashSet;
use stratum_core::{
    edge::EdgeKind,
    ids::{ModuleId, RuleId},
    severity::Severity,
    types::{Bindings, Import},
    violation::{SourceLocation, Violation},
};
use stratum_graph::{Breaks, CompoundGraph, Cut, Cycle, break_candidates_where, find_cycles_where};

use crate::ids::NO_CIRCULAR_DEPS;
use crate::options::NoCircularDepsOptions;
use crate::rule::Rule;
use crate::scope::ProjectScope;

const ANCHOR_NAME_CHARS: usize = 120;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoCircularDeps;

impl Rule for NoCircularDeps {
    type Scope = ProjectScope;
    type Options = NoCircularDepsOptions;

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
        options: &NoCircularDepsOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let deps = Deps::of(graph, *options);
        let mut out: Vec<Violation> = find_cycles_where(graph, |a, b| deps.keeps(a, b))
            .iter()
            .map(|cycle| self.finding(&deps, cycle, severity))
            .collect();
        out.sort_by(|a, b| a.file.cmp(&b.file));
        out
    }
}

/// The dependencies cycles are looked for in: every one, or with an `ignore_*` option on,
/// those with at least one import the options keep.
struct Deps<'g> {
    graph: &'g CompoundGraph,
    options: NoCircularDepsOptions,
    kept: Option<FxHashSet<(ModuleId, ModuleId)>>,
}

impl<'g> Deps<'g> {
    fn of(graph: &'g CompoundGraph, options: NoCircularDepsOptions) -> Self {
        let kept = (options.ignore_type_only || options.ignore_dynamic).then(|| {
            graph
                .modules
                .values()
                .flat_map(|m| m.imports.iter().map(move |i| (m.id, i)))
                .filter(|(_, i)| keeps(options, i))
                .filter_map(|(from, i)| Some((from, i.target?)))
                .collect()
        });
        Self {
            graph,
            options,
            kept,
        }
    }

    fn keeps(&self, from: ModuleId, to: ModuleId) -> bool {
        self.kept.as_ref().is_none_or(|k| k.contains(&(from, to)))
    }

    /// Where `from` first imports `to` by an import the options keep.
    fn import_line(&self, from: ModuleId, to: ModuleId) -> Option<u32> {
        self.graph
            .modules
            .get(&from)?
            .imports
            .iter()
            .find(|i| i.target == Some(to) && keeps(self.options, i))
            .map(|i| i.location.line)
    }
}

fn keeps(options: NoCircularDepsOptions, import: &Import) -> bool {
    !(options.ignore_type_only && type_only(import)
        || options.ignore_dynamic && import.kind == EdgeKind::Runtime)
}

/// `import type`, `export type … from`, or every binding type-only.
pub(crate) fn type_only(import: &Import) -> bool {
    import.type_only
        || matches!(&import.bindings, Bindings::Names(names)
            if !names.is_empty() && names.iter().all(|n| n.type_only))
}

impl NoCircularDeps {
    fn finding(self, deps: &Deps<'_>, cycle: &Cycle, severity: Severity) -> Violation {
        let graph = deps.graph;
        let members = cycle.by_path(graph);
        let anchor = members.first().copied();
        let Breaks { cuts, closing } =
            break_candidates_where(graph, cycle, |a, b| deps.keeps(a, b));

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
                .map(|cut| describe(deps, cut, members.len()))
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
                line: anchor.map_or(1, |m| first_import_line(deps, m, &members)),
                column: 1,
            },
            modules,
            edge: None,
            fix: Some(fix),
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

fn describe(deps: &Deps<'_>, cut: &Cut, size: usize) -> String {
    let at = deps
        .import_line(cut.from, cut.to)
        .map(|line| format!(":{line}"))
        .unwrap_or_default();
    format!(
        "`{}{at} -> {}` (frees {} of {size})",
        display(deps.graph, cut.from),
        display(deps.graph, cut.to),
        cut.freed,
    )
}

fn first_import_line(deps: &Deps<'_>, anchor: ModuleId, members: &[ModuleId]) -> u32 {
    let Some(node) = deps.graph.node_for(anchor) else {
        return 1;
    };
    deps.graph
        .deps
        .neighbors(node)
        .map(|next| deps.graph.deps[next])
        .filter(|to| members.contains(to))
        .filter_map(|to| deps.import_line(anchor, to))
        .min()
        .unwrap_or(1)
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
        types::{ImportedName, Module},
        visibility::VisibilityScope,
    };

    #[derive(Clone, Copy)]
    enum Via {
        Value,
        Type,
        TypeNames,
        Dynamic,
    }

    fn build(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
        let edges: Vec<(u32, u32, Via)> = edges.iter().map(|&(a, b)| (a, b, Via::Value)).collect();
        build_via(n, &edges)
    }

    fn build_via(n: u32, edges: &[(u32, u32, Via)]) -> CompoundGraph {
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
                imports: vec![],
                suppressions: vec![],
            };
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
            g.modules.insert(id, m);
        }
        for (k, &(a, b, via)) in edges.iter().enumerate() {
            let na = g.node_index[&ModuleId::new(a)];
            let nb = g.node_index[&ModuleId::new(b)];
            let kind = match via {
                Via::Dynamic => EdgeKind::Runtime,
                _ => EdgeKind::Static,
            };
            let name = |type_only| ImportedName {
                imported: "x".into(),
                type_only,
            };
            let bindings = match via {
                Via::Value => Bindings::Names(vec![name(false)]),
                Via::Type => Bindings::Names(vec![name(true)]),
                Via::TypeNames => Bindings::Names(vec![name(true), name(true)]),
                Via::Dynamic => Bindings::All,
            };
            g.deps.add_edge(na, nb, kind);
            g.modules[&ModuleId::new(a)].imports.push(Import {
                specifier: format!("./m{b}"),
                location: SourceLocation {
                    line: 100 + u32::try_from(k).unwrap(),
                    column: 1,
                },
                kind,
                type_only: matches!(via, Via::Type),
                bindings,
                target: Some(ModuleId::new(b)),
            });
        }
        g
    }

    fn check(g: &CompoundGraph) -> Vec<Violation> {
        check_with(g, false, false)
    }

    fn check_with(
        g: &CompoundGraph,
        ignore_type_only: bool,
        ignore_dynamic: bool,
    ) -> Vec<Violation> {
        let options = NoCircularDepsOptions {
            ignore_type_only,
            ignore_dynamic,
        };
        NoCircularDeps.check(g, ProjectScope, &options, Severity::Error)
    }

    #[test]
    fn ignore_type_only_drops_a_dependency_made_only_of_type_imports() {
        let g = build_via(
            2,
            &[
                (0, 1, Via::Type),
                (0, 1, Via::TypeNames),
                (1, 0, Via::Value),
            ],
        );
        assert_eq!(check(&g).len(), 1);
        assert!(check_with(&g, true, false).is_empty());
        assert_eq!(check_with(&g, false, true).len(), 1);
    }

    #[test]
    fn ignore_dynamic_drops_a_dependency_made_only_of_dynamic_imports() {
        let g = build_via(2, &[(0, 1, Via::Value), (1, 0, Via::Dynamic)]);
        assert!(check_with(&g, false, true).is_empty());
        assert_eq!(check_with(&g, true, false).len(), 1);
    }

    #[test]
    fn one_value_import_keeps_the_dependency_and_names_its_line() {
        let g = build_via(
            2,
            &[
                (0, 1, Via::Type),
                (0, 1, Via::Dynamic),
                (0, 1, Via::Value),
                (1, 0, Via::Value),
            ],
        );
        let v = check_with(&g, true, true);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].location.line, 102);
        assert!(
            v[0].fix
                .as_deref()
                .unwrap()
                .contains("`src/m0.ts:102 -> src/m1.ts`"),
            "{:?}",
            v[0].fix
        );
    }

    #[test]
    fn both_options_drop_a_dependency_of_type_and_dynamic_imports() {
        let g = build_via(
            2,
            &[(0, 1, Via::Type), (0, 1, Via::Dynamic), (1, 0, Via::Value)],
        );
        assert_eq!(check_with(&g, true, false).len(), 1);
        assert_eq!(check_with(&g, false, true).len(), 1);
        assert!(check_with(&g, true, true).is_empty());
    }

    #[test]
    fn break_candidates_count_only_the_kept_dependencies() {
        let g = build_via(
            3,
            &[
                (0, 1, Via::Value),
                (1, 2, Via::Value),
                (2, 0, Via::Value),
                (1, 0, Via::Type),
            ],
        );
        let fix = |v: Vec<Violation>| v[0].fix.clone().unwrap();
        assert!(fix(check(&g)).contains("(frees 1 of 3)"));
        assert_eq!(
            fix(check_with(&g, true, false))
                .matches("(frees 3 of 3)")
                .count(),
            3
        );
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
    fn finding_sits_on_the_anchors_import_and_lists_cuts_with_lines() {
        let g = build(4, &[(3, 2), (2, 1), (1, 0), (0, 3), (1, 3)]);
        let v = &check(&g)[0];

        assert_eq!(v.file, PathBuf::from("src/m0.ts"));
        assert_eq!(v.location.line, 103);
        assert_eq!(
            v.message,
            "Import cycle of 4 modules through m0.ts; about 1 imports close it"
        );
        let fix = v.fix.as_deref().unwrap();
        assert!(
            fix.starts_with("Break it at one of: `src/m2.ts:101 -> src/m1.ts` (frees 4 of 4)"),
            "{fix}"
        );
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
            check(&ring)[0].fix,
            check(&with_chord)[0].fix,
            "the chord must change the candidates"
        );
        assert_eq!(identity(&ring), identity(&with_chord));
    }
}
