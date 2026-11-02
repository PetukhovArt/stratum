use std::path::{Path, PathBuf};
use stratum_core::text::{Key, Text};

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
    message: Text,
    fix: Text,
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
        message: Text::new(Key::LayerBreadth)
            .arg("name", &layer.name)
            .arg("items", items)
            .arg("max", options.max_items),
        fix: Text::new(Key::LayerBreadthFix),
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
        Key::KindsBreadthToStage3
    } else if c.stage < Stage::COMPOSED {
        Key::KindsBreadthToStage4
    } else {
        Key::KindsBreadthAtStage4
    };
    Some(Finding {
        dir: c.path.clone(),
        entry: c.entry,
        message: Text::new(Key::KindsBreadth)
            .arg("name", &c.name)
            .arg("stage", c.stage.rank())
            .arg("kinds", kinds)
            .arg("max", options.max_kinds)
            .arg("next", next),
        fix: Text::new(Key::KindsBreadthFix),
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
        message: Text::new(Key::ComposerBreadth)
            .arg("name", &c.name)
            .arg("files", files)
            .arg("max", options.max_root_files),
        fix: Text::new(Key::ComposerBreadthFix).arg("name", &c.name),
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
        message: Text::new(Key::FolderBreadth)
            .arg("path", &path)
            .arg("name", &c.name)
            .arg("items", items)
            .arg("max", options.max_items),
        fix: Text::new(Key::FolderBreadthFix).arg("path", &path),
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
        message: Text::new(Key::SingleFile)
            .arg("name", &name)
            .arg("file", file),
        fix: Text::new(Key::SingleFileFix)
            .arg("file", file)
            .arg("name", &name),
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
