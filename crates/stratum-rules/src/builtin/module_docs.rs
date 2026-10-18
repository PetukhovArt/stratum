use std::collections::BTreeSet;
use std::path::Path;

use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::{CompoundGraph, DirRole};

use super::folder::at_folder;
use crate::ids::MODULE_DOCS;
use crate::options::ModuleDocsOptions;
use crate::paths::project_relative;
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct ModuleDocs;

impl Rule for ModuleDocs {
    type Scope = ProjectScope;
    type Options = ModuleDocsOptions;

    fn id(&self) -> RuleId {
        MODULE_DOCS
    }
    fn slug(&self) -> &'static str {
        "stratum/module-docs"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        options: &ModuleDocsOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(nav) = options.nav_file.as_deref() else {
            return vec![];
        };
        let report = |dir: &Path, message: String, fix: String| {
            at_folder(graph, MODULE_DOCS, severity, dir, None, message, fix)
        };
        let mut found = Vec::new();
        for (dir, role) in &graph.dir_roles {
            let name = label(graph, dir);
            let has_doc = graph.nav_docs.contains_key(dir);
            let violation = if role.expects_nav_doc() && !has_doc {
                report(
                    dir,
                    format!("Folder '{name}' has no navigation doc '{nav}'"),
                    format!(
                        "Add '{nav}' to '{name}': what it holds, its public surface and links to its parts."
                    ),
                )
            } else if !role.expects_nav_doc() && has_doc {
                report(
                    dir,
                    format!(
                        "Navigation doc '{nav}' sits in '{name}', a {}, which is no unit",
                        kind_of(*role)
                    ),
                    format!(
                        "Move its content into the navigation doc of the folder that owns '{name}' and delete '{nav}' here."
                    ),
                )
            } else {
                None
            };
            found.extend(violation);
        }
        found.extend(requires_violations(graph, nav, &report));
        found
    }
}

fn kind_of(role: DirRole) -> &'static str {
    match role {
        DirRole::LayerRoot => "layer root",
        DirRole::Container(_) => "container",
        DirRole::Segment { .. } => "segment folder",
        DirRole::PrivateZone => "private zone",
        DirRole::Collection => "collection folder",
    }
}

fn label(graph: &CompoundGraph, dir: &Path) -> String {
    graph
        .first_module_in(dir)
        .and_then(|id| graph.modules.get(&id))
        .map_or_else(
            || dir.to_string_lossy().into_owned(),
            |m| project_relative(graph, m.layer, dir),
        )
}

fn requires_violations(
    graph: &CompoundGraph,
    nav: &str,
    report: &impl Fn(&Path, String, String) -> Option<Violation>,
) -> Vec<Violation> {
    if graph.nav_docs.values().all(|doc| doc.requires.is_empty()) {
        return Vec::new();
    }
    let imports = folder_imports(graph);
    let mut found = Vec::new();
    for (dir, doc) in &graph.nav_docs {
        for required in doc
            .requires
            .iter()
            .filter(|r| graph.dir_roles.contains_key(*r))
        {
            let reason = if required.starts_with(dir) || dir.starts_with(required) {
                continue;
            } else if imports.contains(&(required.as_path(), dir.as_path())) {
                "imports it"
            } else if are_siblings(graph, dir, required) {
                "is its sibling"
            } else {
                continue;
            };
            let (a, b) = (label(graph, dir), label(graph, required));
            found.extend(report(
                dir,
                format!("Navigation doc '{nav}' of '{a}' requires '{b}', which {reason}"),
                format!("Drop '{b}' from the requires of '{a}', or move what '{a}' needs from '{b}' into a folder both may depend on: documents follow imports."),
            ));
        }
    }
    found
}

fn are_siblings(graph: &CompoundGraph, a: &Path, b: &Path) -> bool {
    let unit = |dir: &Path| matches!(graph.dir_roles.get(dir), Some(DirRole::Container(_)));
    unit(a) && unit(b) && a.parent() == b.parent()
}

fn folder_imports(graph: &CompoundGraph) -> BTreeSet<(&Path, &Path)> {
    let documented = |id| {
        let path = graph.modules.get(&id)?.path.as_path();
        Some(
            path.ancestors()
                .filter_map(|dir| graph.nav_docs.get_key_value(dir).map(|(k, _)| k.as_path()))
                .collect::<Vec<_>>(),
        )
    };
    let mut pairs = BTreeSet::new();
    for edge in graph.deps.raw_edges() {
        let (Some(from), Some(to)) = (
            documented(graph.deps[edge.source()]),
            documented(graph.deps[edge.target()]),
        ) else {
            continue;
        };
        for &importer in &from {
            for &imported in &to {
                pairs.insert((importer, imported));
            }
        }
    }
    pairs
}
