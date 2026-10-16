use rustc_hash::FxHashSet;
use stratum_core::{
    ids::ModuleId,
    types::{Bindings, ExportDecl, ExportOrigin, Module},
};

use crate::CompoundGraph;

#[must_use]
pub fn unreachable_modules(graph: &CompoundGraph) -> Vec<ModuleId> {
    let reached = reachable(graph);
    graph
        .modules
        .keys()
        .filter(|id| !reached.contains(id))
        .copied()
        .collect()
}

/// Exports of reachable modules that no reachable importer takes, in graph order. A re-export
/// alone does not take the name it forwards; an entry point's exports and the default export of
/// an imported `.vue` file are taken.
#[must_use]
pub fn unused_exports(graph: &CompoundGraph) -> Vec<(ModuleId, &ExportDecl)> {
    let reached = reachable(graph);
    let mut usage = Usage::new(graph);
    for &entry in &graph.entry_points {
        usage.take(entry, Demand::All);
    }
    for id in &reached {
        let Some(m) = graph.modules.get(id) else {
            continue;
        };
        let forwarding = forwarding_imports(m);
        for (index, import) in m.imports.iter().enumerate() {
            let Some(target) = import.target else {
                continue;
            };
            if is_vue(graph, target) {
                usage.take(target, Demand::Name("default"));
            }
            if forwarding.contains(&index) {
                continue;
            }
            match &import.bindings {
                Bindings::Names(names) => {
                    for n in names {
                        usage.take(target, Demand::Name(&n.imported));
                    }
                }
                Bindings::All => usage.take(target, Demand::All),
                Bindings::None | Bindings::Forward => {}
            }
        }
    }
    usage.run();
    graph
        .modules
        .iter()
        .filter(|(id, _)| reached.contains(id))
        .flat_map(|(&id, m)| {
            let usage = &usage;
            m.exports
                .decls
                .iter()
                .filter(move |d| !usage.is_taken(id, &d.name))
                .map(move |d| (id, d))
        })
        .collect()
}

fn reachable(graph: &CompoundGraph) -> FxHashSet<ModuleId> {
    let mut seen: FxHashSet<ModuleId> = FxHashSet::default();
    let mut stack: Vec<ModuleId> = graph.entry_points.clone();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if let Some(m) = graph.modules.get(&id) {
            stack.extend(m.imports.iter().filter_map(|i| i.target));
        }
    }
    seen
}

fn forwarding_imports(m: &Module) -> FxHashSet<usize> {
    let mut indices: FxHashSet<usize> = m.exports.stars.iter().copied().collect();
    indices.extend(m.exports.decls.iter().filter_map(|d| match d.origin {
        ExportOrigin::Reexport { import, .. } => Some(import),
        ExportOrigin::Local => None,
    }));
    indices
}

fn is_vue(graph: &CompoundGraph, id: ModuleId) -> bool {
    graph
        .modules
        .get(&id)
        .is_some_and(|m| m.path.extension().is_some_and(|e| e == "vue"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Demand<'g> {
    All,
    Name(&'g str),
}

struct Usage<'g> {
    graph: &'g CompoundGraph,
    queue: Vec<(ModuleId, Demand<'g>)>,
    seen: FxHashSet<(ModuleId, Demand<'g>)>,
}

impl<'g> Usage<'g> {
    fn new(graph: &'g CompoundGraph) -> Self {
        Self {
            graph,
            queue: Vec::new(),
            seen: FxHashSet::default(),
        }
    }

    fn take(&mut self, module: ModuleId, demand: Demand<'g>) {
        if self.seen.insert((module, demand)) {
            self.queue.push((module, demand));
        }
    }

    fn is_taken(&self, module: ModuleId, name: &str) -> bool {
        self.seen.contains(&(module, Demand::All))
            || self.seen.contains(&(module, Demand::Name(name)))
    }

    fn run(&mut self) {
        let graph = self.graph;
        while let Some((id, demand)) = self.queue.pop() {
            let Some(m) = graph.modules.get(&id) else {
                continue;
            };
            let target_of = |import: usize| m.imports.get(import).and_then(|i| i.target);
            match demand {
                Demand::All => {
                    for d in &m.exports.decls {
                        self.forward(d, &target_of);
                    }
                    for &star in &m.exports.stars {
                        if let Some(t) = target_of(star) {
                            self.take(t, Demand::All);
                        }
                    }
                }
                Demand::Name(name) => {
                    if let Some(d) = m.exports.decls.iter().find(|d| d.name == name) {
                        self.forward(d, &target_of);
                    } else if name != "default" {
                        for &star in &m.exports.stars {
                            if let Some(t) = target_of(star) {
                                self.take(t, Demand::Name(name));
                            }
                        }
                    }
                }
            }
        }
    }

    fn forward(&mut self, decl: &'g ExportDecl, target_of: &impl Fn(usize) -> Option<ModuleId>) {
        if let ExportOrigin::Reexport { import, imported } = &decl.origin
            && let Some(t) = target_of(*import)
        {
            let demand = if imported == "*" {
                Demand::All
            } else {
                Demand::Name(imported)
            };
            self.take(t, demand);
        }
    }
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
        types::{Exports, Import, ImportedName},
        violation::SourceLocation,
        visibility::VisibilityScope,
    };

    const AT: SourceLocation = SourceLocation { line: 1, column: 1 };

    fn module(id: u32, exports: &[&str], imports: Vec<Import>, stars: Vec<usize>) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(1),
            layer: LayerId::new(1),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: Exports {
                decls: exports
                    .iter()
                    .map(|name| ExportDecl {
                        name: (*name).to_owned(),
                        type_only: false,
                        location: AT,
                        origin: ExportOrigin::Local,
                    })
                    .collect(),
                stars,
                unimportable: 0,
            },
            calls: vec![],
            imports,
            suppressions: vec![],
        }
    }

    fn import(target: u32, bindings: Bindings) -> Import {
        Import {
            specifier: format!("./m{target}"),
            location: AT,
            kind: EdgeKind::Static,
            type_only: false,
            bindings,
            target: Some(ModuleId::new(target)),
        }
    }

    #[test]
    fn a_name_imported_through_an_export_star_barrel_is_used() {
        let used = Bindings::Names(vec![ImportedName {
            imported: "used".to_owned(),
            type_only: false,
        }]);
        let mut g = CompoundGraph::empty();
        for m in [
            module(0, &[], vec![import(1, used)], vec![]),
            module(1, &[], vec![import(2, Bindings::Forward)], vec![0]),
            module(2, &["used", "unused"], vec![], vec![]),
        ] {
            g.modules.insert(m.id, m);
        }
        g.entry_points = vec![ModuleId::new(0)];
        let unused: Vec<(u32, &str)> = unused_exports(&g)
            .into_iter()
            .map(|(id, d)| (id.raw(), d.name.as_str()))
            .collect();
        assert_eq!(unused, [(2, "unused")]);
    }
}
