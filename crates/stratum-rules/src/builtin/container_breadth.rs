use std::path::{Path, PathBuf};

use stratum_core::{
    ids::{ContainerId, ModuleId, RuleId},
    severity::Severity,
    stage::Stage,
    types::{Container, ContainerKind},
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use super::folder::{at_folder, relative};
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
        let mut found = Vec::new();
        if c.kind == ContainerKind::Layer {
            found.extend(sliced_layer(graph, c, options));
        } else if c.stage != Stage::FILE {
            found.extend(heterogeneous(graph, c, options));
            found.extend(composer(graph, c, options));
            found.extend(single_file(graph, &c.path, c.entry));
        }
        for (dir, role) in graph.folders_below(scope) {
            if role.is_segment_or_collection() {
                found.extend(homogeneous(graph, c, dir, options));
                found.extend(single_file(graph, dir, None));
            }
        }
        found
            .into_iter()
            .filter_map(|f| {
                let severity = if f.single_file {
                    severity.min(Severity::Info)
                } else {
                    severity
                };
                at_folder(
                    graph,
                    CONTAINER_BREADTH,
                    severity,
                    &f.dir,
                    f.entry,
                    f.message,
                    f.fix,
                )
            })
            .collect()
    }
}

struct Finding {
    dir: PathBuf,
    entry: Option<ModuleId>,
    message: String,
    fix: String,
    single_file: bool,
}

fn limit(max: u32) -> usize {
    usize::try_from(max).unwrap_or(usize::MAX)
}

fn sliced_layer(
    graph: &CompoundGraph,
    layer: &Container,
    options: &ContainerBreadthOptions,
) -> Option<Finding> {
    let sliced = graph.layers.get(&layer.layer)?.slices;
    let items = graph.items(&layer.path);
    (sliced && items > limit(options.max_items)).then(|| Finding {
        dir: layer.path.clone(),
        entry: layer.entry,
        message: format!(
            "Layer '{name}' holds {items} slices and files directly inside it (limit {max})",
            name = layer.name,
            max = options.max_items,
        ),
        fix: "Group related slices under a shared parent slice, or move the ones several layers use into a lower layer.".into(),
        single_file: false,
    })
}

fn heterogeneous(
    graph: &CompoundGraph,
    c: &Container,
    options: &ContainerBreadthOptions,
) -> Option<Finding> {
    let kinds = graph.kinds(&c.path).len();
    if kinds <= limit(options.max_kinds) {
        return None;
    }
    let next = if c.stage < Stage::SEGMENTED {
        "evolve it to stage 3 by grouping files into segment folders"
    } else if c.stage < Stage::COMPOSED {
        "evolve it to stage 4 by splitting it into sub-units with their own entry file"
    } else {
        "it is at stage 4 already: group its sub-units into fewer, larger ones"
    };
    Some(Finding {
        dir: c.path.clone(),
        entry: c.entry,
        message: format!(
            "Container '{name}' (stage {stage}) holds {kinds} kinds directly inside it (limit {max}); {next}",
            name = c.name,
            stage = c.stage.rank(),
            max = options.max_kinds,
        ),
        fix: "Group the files into segment folders (stage 3), or split them into sub-units with their own entry file (stage 4).".into(),
        single_file: false,
    })
}

fn composer(
    graph: &CompoundGraph,
    c: &Container,
    options: &ContainerBreadthOptions,
) -> Option<Finding> {
    let files = graph.loose_files(&c.path);
    (c.stage == Stage::COMPOSED && files > limit(options.max_root_files)).then(|| Finding {
        dir: c.path.clone(),
        entry: c.entry,
        message: format!(
            "Composer '{name}' holds {files} files at its root besides its entry (limit {max}); a composer only wires its sub-units",
            name = c.name,
            max = options.max_root_files,
        ),
        fix: format!(
            "Move the logic in the root files of '{name}' into its sub-units or a new sub-unit, and keep only the wiring at the root.",
            name = c.name,
        ),
        single_file: false,
    })
}

fn homogeneous(
    graph: &CompoundGraph,
    c: &Container,
    dir: &Path,
    options: &ContainerBreadthOptions,
) -> Option<Finding> {
    let items = graph.items(dir);
    let path = relative(dir, &c.path);
    (items > limit(options.max_items)).then(|| Finding {
        dir: dir.to_path_buf(),
        entry: None,
        message: format!(
            "Folder '{path}' of '{name}' holds {items} items directly inside it (limit {max})",
            name = c.name,
            max = options.max_items,
        ),
        fix: format!(
            "Split '{path}' into sub-folders by theme, or into sub-units with their own entry file."
        ),
        single_file: false,
    })
}

fn single_file(graph: &CompoundGraph, dir: &Path, entry: Option<ModuleId>) -> Option<Finding> {
    let entries = graph.dir_entries(dir)?;
    let [file] = entries.files.as_slice() else {
        return None;
    };
    if !entries.dirs.is_empty() {
        return None;
    }
    let name = dir
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    Some(Finding {
        dir: dir.to_path_buf(),
        entry,
        message: format!("Folder '{name}' holds a single file, '{file}'"),
        fix: format!(
            "Move '{file}' up into the parent folder and delete '{name}', unless more files are about to join it."
        ),
        single_file: true,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
                    imports: vec![],
                    suppressions: vec![],
                    local_refs: vec![],
                },
            );
        }
        g.dir_roles.insert(
            PathBuf::from(format!("{SLICE}/_shared")),
            stratum_graph::DirRole::PrivateZone,
        );
        g
    }

    fn check(g: &CompoundGraph, severity: Severity) -> Vec<Violation> {
        let options = ContainerBreadthOptions {
            max_kinds: 2,
            ..ContainerBreadthOptions::default()
        };
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
}
