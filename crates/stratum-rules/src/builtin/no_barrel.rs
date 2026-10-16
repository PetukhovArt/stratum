use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    types::ContainerKind,
    violation::Violation,
};
use stratum_graph::{CompoundGraph, DirRole};

use super::folder::at_module;
use crate::ids::NO_BARREL;
use crate::options::NoBarrelOptions;
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoBarrel;

impl Rule for NoBarrel {
    type Scope = ProjectScope;
    type Options = NoBarrelOptions;

    fn id(&self) -> RuleId {
        NO_BARREL
    }
    fn slug(&self) -> &'static str {
        "stratum/no-barrel"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        options: &NoBarrelOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let in_segments = graph.dir_roles.iter().filter_map(|(dir, role)| {
            let DirRole::Segment { entry: Some(entry) } = role else {
                return None;
            };
            let segment = dir.file_name()?.to_string_lossy();
            let file = file_name(graph, *entry)?;
            Some((
                *entry,
                format!("Entry file '{file}' sits directly in segment folder '{segment}'"),
                format!(
                    "Delete '{file}' and import the files of '{segment}' directly; the unit that owns the segment exposes its public surface through its own entry file."
                ),
            ))
        });
        let at_layer_roots = graph
            .containers
            .values()
            .filter(|c| c.kind == ContainerKind::Layer)
            .filter(|c| {
                graph
                    .layers
                    .get(&c.layer)
                    .is_some_and(|l| options.layers.contains(&l.name))
            })
            .filter_map(|c| {
                let entry = c.entry?;
                let file = file_name(graph, entry)?;
                Some((
                    entry,
                    format!("Entry file '{file}' sits at the root of layer '{}'", c.name),
                    format!(
                        "Delete '{file}' and import from the files, or from a sub-unit's entry file, directly."
                    ),
                ))
            });
        in_segments
            .chain(at_layer_roots)
            .filter_map(|(entry, message, fix)| {
                at_module(graph, NO_BARREL, severity, entry, message, fix)
            })
            .collect()
    }
}

fn file_name(graph: &CompoundGraph, module: ModuleId) -> Option<String> {
    let name = graph.modules.get(&module)?.path.file_name()?;
    Some(name.to_string_lossy().into_owned())
}
