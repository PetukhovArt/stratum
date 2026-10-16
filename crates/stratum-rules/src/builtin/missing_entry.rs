use std::path::Path;

use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::{CompoundGraph, DirRole};

use super::folder::{at_folder, entry_file_phrase};
use crate::ids::MISSING_ENTRY;
use crate::rule::{EmptyOptions, Rule};
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct MissingEntry;

impl Rule for MissingEntry {
    type Scope = ProjectScope;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        MISSING_ENTRY
    }
    fn slug(&self) -> &'static str {
        "stratum/missing-entry"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let entry = entry_file_phrase(graph);
        graph
            .dir_roles
            .iter()
            .filter(|(_, role)| lacks_entry(graph, **role))
            .filter_map(|(dir, _)| {
                let parts = internal_structure(graph, dir);
                if parts.is_empty() {
                    return None;
                }
                let name = dir.file_name()?.to_string_lossy();
                at_folder(
                    graph,
                    MISSING_ENTRY,
                    severity,
                    dir,
                    None,
                    format!(
                        "Folder '{name}' has internal structure ({parts}) but no entry file",
                        parts = parts.join(", "),
                    ),
                    format!(
                        "Add {entry} to '{name}' exporting its public surface, or flatten the folder if it is not a unit."
                    ),
                )
            })
            .collect()
    }
}

fn lacks_entry(graph: &CompoundGraph, role: DirRole) -> bool {
    match role {
        DirRole::Collection => true,
        DirRole::Container(id) => graph.containers.get(&id).is_some_and(|c| c.entry.is_none()),
        DirRole::LayerRoot | DirRole::Segment { .. } | DirRole::PrivateZone => false,
    }
}

fn internal_structure(graph: &CompoundGraph, dir: &Path) -> Vec<String> {
    let Some(entries) = graph.dir_entries(dir) else {
        return vec![];
    };
    entries
        .dirs
        .iter()
        .filter(|name| {
            matches!(
                graph.dir_roles.get(&dir.join(name)),
                Some(DirRole::Segment { .. } | DirRole::PrivateZone)
            )
        })
        .cloned()
        .collect()
}
