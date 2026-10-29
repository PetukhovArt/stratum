use rustc_hash::FxHashSet;
use stratum_core::{
    ids::ModuleId,
    types::{Bindings, ExportOrigin, Import},
    violation::SourceLocation,
};

use crate::CompoundGraph;

/// A name that crosses an import edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossingName {
    /// The name the target exports, `default` for a default import, `*` for the whole module.
    pub name: String,
    /// Set by `import type` or by an inline `type` modifier.
    pub type_only: bool,
    pub decl: Option<Declaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub module: ModuleId,
    /// The top-level binding of `module` the name stands for; `None` for the whole module.
    pub binding: Option<String>,
    pub location: SourceLocation,
}

const MODULE_START: SourceLocation = SourceLocation { line: 1, column: 1 };

/// The names `import` takes from its target, each followed through re-exports and
/// `export *` to the module that declares it. `decl` is `None` for a package target, a name
/// no module declares, and a re-export cycle. A namespace or forwarding import takes `*`,
/// declared by the target at its first line; a side-effect import takes nothing.
#[must_use]
pub fn crossing_names(graph: &CompoundGraph, import: &Import) -> Vec<CrossingName> {
    let crossing = |name: &str, type_only: bool, decl: Option<Declaration>| CrossingName {
        name: name.to_owned(),
        type_only: import.type_only || type_only,
        decl,
    };
    match &import.bindings {
        Bindings::Names(names) => names
            .iter()
            .map(|n| {
                let decl = import
                    .target
                    .and_then(|t| declaration(graph, t, &n.imported, &mut FxHashSet::default()));
                crossing(&n.imported, n.type_only, decl)
            })
            .collect(),
        Bindings::All | Bindings::Forward => {
            let decl = import
                .target
                .filter(|t| graph.modules.contains_key(t))
                .map(|module| Declaration {
                    module,
                    binding: None,
                    location: MODULE_START,
                });
            vec![crossing("*", false, decl)]
        }
        Bindings::None => vec![],
    }
}

fn declaration<'g>(
    graph: &'g CompoundGraph,
    module: ModuleId,
    name: &'g str,
    seen: &mut FxHashSet<(ModuleId, &'g str)>,
) -> Option<Declaration> {
    if !seen.insert((module, name)) {
        return None;
    }
    let m = graph.modules.get(&module)?;
    let target_of = |import: usize| m.imports.get(import).and_then(|i| i.target);
    if let Some(d) = m.exports.decls.iter().find(|d| d.name == name) {
        return match &d.origin {
            ExportOrigin::Local => Some(Declaration {
                module,
                binding: Some(d.name.clone()),
                location: d.location,
            }),
            ExportOrigin::Alias { binding } => Some(Declaration {
                module,
                binding: Some(binding.clone()),
                location: d.location,
            }),
            ExportOrigin::Reexport { import, imported } => {
                let target = target_of(*import)?;
                if imported == "*" {
                    graph.modules.contains_key(&target).then_some(Declaration {
                        module: target,
                        binding: None,
                        location: MODULE_START,
                    })
                } else {
                    declaration(graph, target, imported, seen)
                }
            }
        };
    }
    if name == "default" {
        return None;
    }
    m.exports
        .stars
        .iter()
        .filter_map(|&star| target_of(star))
        .find_map(|t| declaration(graph, t, name, seen))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::{ExportDecl, Exports, ImportedName, Module},
        visibility::VisibilityScope,
    };

    fn at(line: u32) -> SourceLocation {
        SourceLocation { line, column: 1 }
    }

    fn local(name: &str, line: u32) -> ExportDecl {
        ExportDecl {
            name: name.to_owned(),
            type_only: false,
            location: at(line),
            origin: ExportOrigin::Local,
        }
    }

    fn reexport(name: &str, import: usize, imported: &str) -> ExportDecl {
        ExportDecl {
            name: name.to_owned(),
            type_only: false,
            location: at(1),
            origin: ExportOrigin::Reexport {
                import,
                imported: imported.to_owned(),
            },
        }
    }

    fn import(target: Option<u32>, bindings: Bindings) -> Import {
        Import {
            specifier: "./x".to_owned(),
            location: at(1),
            kind: EdgeKind::Static,
            type_only: false,
            bindings,
            target: target.map(ModuleId::new),
        }
    }

    fn names(names: &[&str]) -> Bindings {
        Bindings::Names(
            names
                .iter()
                .map(|n| ImportedName {
                    imported: (*n).to_owned(),
                    type_only: false,
                })
                .collect(),
        )
    }

    type Spec = (u32, Vec<ExportDecl>, Vec<Import>, Vec<usize>);

    fn graph(modules: Vec<Spec>) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for (id, decls, imports, stars) in modules {
            let m = Module {
                id: ModuleId::new(id),
                path: PathBuf::from(format!("m{id}.ts")),
                container: ContainerId::new(1),
                layer: LayerId::new(1),
                purity: Purity::new(2).unwrap(),
                visibility: VisibilityScope::Public,
                exports: Exports {
                    decls,
                    stars,
                    unimportable: 0,
                },
                calls: vec![],
                imports,
                suppressions: vec![],
                local_refs: vec![],
            };
            g.modules.insert(m.id, m);
        }
        g
    }

    fn resolved(g: &CompoundGraph, import: &Import) -> Vec<(String, Option<(u32, u32)>)> {
        crossing_names(g, import)
            .into_iter()
            .map(|n| (n.name, n.decl.map(|d| (d.module.raw(), d.location.line))))
            .collect()
    }

    #[test]
    fn a_named_reexport_chain_resolves_to_the_declaring_module_under_its_original_name() {
        let g = graph(vec![
            (
                1,
                vec![reexport("Api", 0, "Inner")],
                vec![import(Some(2), names(&["Inner"]))],
                vec![],
            ),
            (
                2,
                vec![reexport("Inner", 0, "Impl")],
                vec![import(Some(3), names(&["Impl"]))],
                vec![],
            ),
            (3, vec![local("Other", 2), local("Impl", 7)], vec![], vec![]),
        ]);
        let edge = import(Some(1), names(&["Api"]));
        assert_eq!(resolved(&g, &edge), [("Api".to_owned(), Some((3, 7)))]);
        let binding = crossing_names(&g, &edge)[0]
            .decl
            .as_ref()
            .unwrap()
            .binding
            .clone();
        assert_eq!(binding.as_deref(), Some("Impl"));
    }

    #[test]
    fn an_export_star_barrel_resolves_a_name_from_whichever_star_target_declares_it() {
        let g = graph(vec![
            (
                1,
                vec![],
                vec![
                    import(Some(2), Bindings::Forward),
                    import(Some(3), Bindings::Forward),
                ],
                vec![0, 1],
            ),
            (2, vec![local("a", 4)], vec![], vec![]),
            (3, vec![local("b", 9), local("default", 12)], vec![], vec![]),
        ]);
        let edge = import(Some(1), names(&["b", "a", "default"]));
        assert_eq!(
            resolved(&g, &edge),
            [
                ("b".to_owned(), Some((3, 9))),
                ("a".to_owned(), Some((2, 4))),
                ("default".to_owned(), None),
            ]
        );
    }

    #[test]
    fn an_export_star_as_namespace_resolves_to_the_target_module_start() {
        let g = graph(vec![
            (
                1,
                vec![reexport("ns", 0, "*")],
                vec![import(Some(2), Bindings::All)],
                vec![],
            ),
            (2, vec![local("x", 5)], vec![], vec![]),
        ]);
        let edge = import(Some(1), names(&["ns"]));
        assert_eq!(resolved(&g, &edge), [("ns".to_owned(), Some((2, 1)))]);
    }

    #[test]
    fn a_reexport_cycle_or_a_missing_name_leaves_the_name_unresolved() {
        let g = graph(vec![
            (
                1,
                vec![reexport("loop", 0, "loop")],
                vec![import(Some(2), names(&["loop"]))],
                vec![],
            ),
            (2, vec![], vec![import(Some(1), Bindings::Forward)], vec![0]),
        ]);
        let edge = import(Some(1), names(&["loop", "absent"]));
        assert_eq!(
            resolved(&g, &edge),
            [("loop".to_owned(), None), ("absent".to_owned(), None)]
        );
    }

    #[test]
    fn whole_module_imports_take_star_and_side_effect_imports_take_nothing() {
        let g = graph(vec![(1, vec![local("x", 3)], vec![], vec![])]);
        for bindings in [Bindings::All, Bindings::Forward] {
            let edge = import(Some(1), bindings);
            assert_eq!(resolved(&g, &edge), [("*".to_owned(), Some((1, 1)))]);
        }
        assert_eq!(resolved(&g, &import(Some(1), Bindings::None)), []);
        assert_eq!(
            resolved(&g, &import(None, names(&["x"]))),
            [("x".to_owned(), None)]
        );
    }
}
