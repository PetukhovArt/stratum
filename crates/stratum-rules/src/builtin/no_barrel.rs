use stratum_core::text::{Key, Text};
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
                Text::new(Key::BarrelInSegment)
                    .arg("file", &file)
                    .arg("segment", &*segment),
                Text::new(Key::BarrelInSegmentFix)
                    .arg("file", file)
                    .arg("segment", &*segment),
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
                    Text::new(Key::BarrelAtLayerRoot)
                        .arg("file", &file)
                        .arg("layer", &c.name),
                    Text::new(Key::BarrelAtLayerRootFix).arg("file", file),
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
