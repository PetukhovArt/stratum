use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use stratum_core::{
    ids::{ContainerId, ModuleId, RuleId},
    severity::Severity,
    stage::Stage,
    types::{Container, ContainerKind},
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::CONTAINER_BREADTH;
use crate::options::ContainerBreadthOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct ContainerBreadth;

impl Rule for ContainerBreadth {
    type Scope = ContainerId;
    type Options = ContainerBreadthOptions;

    fn id(&self) -> RuleId {
        CONTAINER_BREADTH
    }
    fn slug(&self) -> &'static str {
        "stratum/container-breadth"
    }
    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ContainerId,
        options: &ContainerBreadthOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(c) = graph.containers.get(&scope) else {
            return vec![];
        };
        let zones = graph.private_zones();
        let zone_paths: BTreeSet<&Path> = zones.iter().map(|z| z.path.as_path()).collect();
        let mut out = Vec::new();
        if c.kind != ContainerKind::Layer && c.stage != Stage::FILE {
            let kinds = kinds_in(graph, &c.path, c.entry, &zone_paths);
            if kinds.len() > limit(options.max_kinds) {
                let rep = c.entry.or_else(|| first_module_in(graph, scope));
                out.extend(rep.and_then(|rep| {
                    violation(
                        graph,
                        rep,
                        severity,
                        container_message(c, kinds.len(), options.max_kinds),
                    )
                }));
            }
        }
        for zone in &zones {
            if graph.container_at(&zone.path).map(|z| z.id) != Some(scope) {
                continue;
            }
            let kinds = kinds_in(graph, &zone.path, None, &zone_paths);
            if kinds.len() <= limit(options.max_kinds) {
                continue;
            }
            let message = format!(
                "Private zone '{zone}' of '{name}' holds {n} kinds directly inside it (limit {max}); group its files into folders",
                zone = display(zone.path.strip_prefix(&c.path).unwrap_or(&zone.path)),
                name = c.name,
                n = kinds.len(),
                max = options.max_kinds,
            );
            out.extend(
                zone.files
                    .first()
                    .and_then(|rep| violation(graph, *rep, severity.min(Severity::Info), message)),
            );
        }
        out
    }
}

fn violation(
    graph: &CompoundGraph,
    rep: ModuleId,
    severity: Severity,
    message: String,
) -> Option<Violation> {
    let file = graph.modules.get(&rep)?.path.clone();
    Some(Violation {
        rule: CONTAINER_BREADTH,
        severity,
        message,
        file,
        location: SourceLocation { line: 1, column: 1 },
        modules: vec![rep],
        edge: None,
        suggestion: Some(
            "Group the files into segment folders (stage 3), or split them into sub-units with their own entry file (stage 4).".into(),
        ),
    })
}

fn limit(max_kinds: u32) -> usize {
    usize::try_from(max_kinds).unwrap_or(usize::MAX)
}

fn container_message(c: &Container, kinds: usize, max_kinds: u32) -> String {
    let next = if c.stage < Stage::SEGMENTED {
        "evolve it to stage 3 by grouping files into segment folders"
    } else if c.stage < Stage::COMPOSED {
        "evolve it to stage 4 by splitting it into sub-units with their own entry file"
    } else {
        "it is at stage 4 already: group its sub-units into fewer, larger ones"
    };
    format!(
        "Container '{name}' (stage {stage}) holds {kinds} kinds directly inside it (limit {max}); {next}",
        name = c.name,
        stage = c.stage.rank(),
        max = max_kinds,
    )
}

fn first_module_in(graph: &CompoundGraph, c: ContainerId) -> Option<ModuleId> {
    graph
        .modules
        .values()
        .find(|m| m.container == c)
        .map(|m| m.id)
}

fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn kinds_in(
    graph: &CompoundGraph,
    dir: &Path,
    entry: Option<ModuleId>,
    zones: &BTreeSet<&Path>,
) -> BTreeSet<String> {
    let entry_stem = entry
        .and_then(|e| graph.modules.get(&e))
        .and_then(|m| m.path.file_name())
        .and_then(|n| stem(&n.to_string_lossy()));
    let mut kinds = BTreeSet::new();
    for m in graph.modules.values() {
        let Ok(rest) = m.path.strip_prefix(dir) else {
            continue;
        };
        let mut parts = rest.iter();
        let Some(first) = parts.next() else {
            continue;
        };
        let first = first.to_string_lossy();
        if parts.next().is_some() {
            let folder: PathBuf = dir.join(first.as_ref());
            if !zones.contains(folder.as_path()) {
                kinds.insert(format!("{first}/"));
            }
        } else if let Some(kind) = stem(&first)
            && Some(&kind) != entry_stem.as_ref()
        {
            kinds.insert(kind);
        }
    }
    kinds
}

fn stem(file_name: &str) -> Option<String> {
    file_name
        .split('.')
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use stratum_core::{ids::LayerId, purity::Purity, types::Module, visibility::VisibilityScope};

    const SLICE: &str = "src/features/cart";

    fn graph(files: &[&str], entry: Option<u32>) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for (id, path, kind, parent) in [
            (0, "src/features", ContainerKind::Layer, None),
            (1, SLICE, ContainerKind::Slice, Some(0)),
        ] {
            let id = ContainerId::new(id);
            g.containers.insert(
                id,
                Container {
                    id,
                    name: path.rsplit('/').next().unwrap().into(),
                    path: PathBuf::from(path),
                    kind,
                    layer: LayerId::new(0),
                    parent: parent.map(ContainerId::new),
                    stage: Stage::FLAT,
                    entry: if id.raw() == 1 {
                        entry.map(ModuleId::new)
                    } else {
                        None
                    },
                    segment_dirs: vec![],
                },
            );
        }
        for (i, file) in files.iter().enumerate() {
            let id = ModuleId::new(u32::try_from(i).unwrap());
            let path = format!("{SLICE}/{file}");
            let visibility = path
                .find("/_shared/")
                .map_or(VisibilityScope::Public, |at| VisibilityScope::Private {
                    root: PathBuf::from(&path[..at]),
                });
            g.modules.insert(
                id,
                Module {
                    id,
                    path: PathBuf::from(path),
                    container: ContainerId::new(1),
                    layer: LayerId::new(0),
                    purity: Purity::new(2).unwrap(),
                    visibility,
                    exports: stratum_core::types::Exports::default(),
                    calls: vec![],
                },
            );
        }
        g
    }

    fn check(g: &CompoundGraph, severity: Severity) -> Vec<Violation> {
        let options = ContainerBreadthOptions { max_kinds: 2 };
        ContainerBreadth.check(g, ContainerId::new(1), &options, severity)
    }

    #[test]
    fn files_collapse_by_stem_and_entry_and_zones_do_not_count() {
        let g = graph(
            &[
                "index.ts",
                "index.test.ts",
                "foo.ts",
                "foo.test.ts",
                "foo.vue",
                "ui/button.vue",
                "_shared/a.ts",
            ],
            Some(0),
        );
        assert!(check(&g, Severity::Warning).is_empty());
    }

    #[test]
    fn too_many_kinds_names_the_next_stage() {
        let g = graph(&["a.ts", "b.ts", "c/x.ts"], None);
        let v = check(&g, Severity::Warning);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].severity, Severity::Warning);
        assert!(v[0].message.contains("stage 3"));
    }

    #[test]
    fn broad_private_zone_is_downgraded_to_info() {
        let g = graph(&["_shared/a.ts", "_shared/b.ts", "_shared/c.ts"], None);
        let v = check(&g, Severity::Error);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].severity, Severity::Info);
        assert!(v[0].message.contains("Private zone"));
    }
}
