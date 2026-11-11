use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globset::GlobSet;
use serde::Deserialize;
use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    text::{Key, Text},
    types::{ContainerKind, Layer},
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, DirRole};

use crate::builtin::import_policy::glob_set;
use crate::ids::UNCLAIMED_FOLDER;
use crate::paths::project_relative;
use crate::rule::Rule;
use crate::scope::ProjectScope;

pub const UNCLAIMED_FOLDER_SLUG: &str = "stratum/unclaimed-folder";

#[derive(Debug, Default, Clone, Copy)]
pub struct UnclaimedFolder;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(try_from = "UnclaimedFolderSpec")]
pub struct UnclaimedFolderOptions {
    layers: Vec<String>,
    allow: GlobSet,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnclaimedFolderSpec {
    #[serde(default)]
    layers: Vec<String>,
    #[serde(default)]
    allow: Vec<String>,
}

impl TryFrom<UnclaimedFolderSpec> for UnclaimedFolderOptions {
    type Error = String;

    fn try_from(spec: UnclaimedFolderSpec) -> Result<Self, String> {
        Ok(Self {
            allow: glob_set(&spec.allow)?,
            layers: spec.layers,
        })
    }
}

impl Rule for UnclaimedFolder {
    type Scope = ProjectScope;
    type Options = UnclaimedFolderOptions;

    fn id(&self) -> RuleId {
        UNCLAIMED_FOLDER
    }
    fn slug(&self) -> &'static str {
        UNCLAIMED_FOLDER_SLUG
    }
    fn default_severity(&self) -> Severity {
        Severity::Off
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        options: &UnclaimedFolderOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let layer_roots: Vec<&Path> = graph
            .containers
            .values()
            .filter(|c| c.kind == ContainerKind::Layer)
            .map(|c| c.path.as_path())
            .collect();
        graph
            .containers
            .values()
            .filter(|c| c.kind == ContainerKind::Layer)
            .filter_map(|c| Some((c.path.as_path(), graph.layers.get(&c.layer)?)))
            .filter(|(_, layer)| options.layers.contains(&layer.name))
            .flat_map(|(root, layer)| {
                folders(graph, root, layer)
                    .into_iter()
                    .filter(|(dir, _)| !claimed(graph, &layer_roots, options, layer, dir))
                    .filter_map(|(dir, modules)| finding(graph, severity, layer, &dir, modules))
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

fn folders(
    graph: &CompoundGraph,
    root: &Path,
    layer: &Layer,
) -> BTreeMap<PathBuf, Vec<(PathBuf, ModuleId)>> {
    let mut out: BTreeMap<PathBuf, Vec<(PathBuf, ModuleId)>> = BTreeMap::new();
    for m in graph.modules.values().filter(|m| m.layer == layer.id) {
        let Ok(inner) = m.path.strip_prefix(root) else {
            continue;
        };
        let mut parts = inner.components();
        if let (Some(child), Some(_)) = (parts.next(), parts.next()) {
            out.entry(root.join(child))
                .or_default()
                .push((m.path.clone(), m.id));
        }
    }
    for modules in out.values_mut() {
        modules.sort();
    }
    out
}

fn claimed(
    graph: &CompoundGraph,
    layer_roots: &[&Path],
    options: &UnclaimedFolderOptions,
    layer: &Layer,
    dir: &Path,
) -> bool {
    layer_roots.iter().any(|root| root.starts_with(dir))
        || matches!(
            graph.dir_roles.get(dir),
            Some(DirRole::PrivateZone | DirRole::Segment { .. })
        )
        || options
            .allow
            .is_match(project_relative(graph, layer.id, dir).as_str())
}

fn finding(
    graph: &CompoundGraph,
    severity: Severity,
    layer: &Layer,
    dir: &Path,
    modules: Vec<(PathBuf, ModuleId)>,
) -> Option<Violation> {
    let (file, _) = modules.first()?.clone();
    let folder = project_relative(graph, layer.id, dir);
    let message = Text::new(Key::UnclaimedFolder)
        .arg("folder", &folder)
        .arg("layer", &layer.name)
        .arg("count", layer.depends_on.len());
    let fix = Text::new(Key::UnclaimedFolderFix).arg("folder", folder);
    Some(Violation {
        rule: UNCLAIMED_FOLDER,
        severity,
        message: message.en(),
        file,
        location: SourceLocation { line: 1, column: 1 },
        modules: modules.into_iter().map(|(_, id)| id).collect(),
        edge: None,
        fix: Some(fix.en()),
        message_text: Some(message),
        fix_text: Some(fix),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::builtin::public_api::fixture::{Fixture, ROOT};
    use stratum_core::ids::LayerId;

    fn editor() -> Fixture {
        let mut f = Fixture::default();
        f.layer(0, "src/features/editor");
        f.layer(1, "src/features/editor/core");
        f.layer(2, "src/features/editor/panels/props");
        f.layer(3, "src/shared");
        for (raw, name) in [(0, "editor-root"), (1, "core"), (2, "props"), (3, "shared")] {
            let layer = f.g.layers.get_mut(&LayerId::new(raw)).unwrap();
            layer.name = name.into();
            layer.slices = false;
        }
        f.g.layers.get_mut(&LayerId::new(0)).unwrap().depends_on =
            vec![LayerId::new(1), LayerId::new(2), LayerId::new(3)];
        for rel in [
            "src/features/editor/index.ts",
            "src/features/editor/core/model.ts",
            "src/features/editor/panels/frame.ts",
            "src/features/editor/panels/props/form.ts",
            "src/features/editor/_shared/scope.ts",
            "src/features/editor/ui/button.ts",
            "src/features/editor/image-picker/picker.ts",
            "src/features/editor/image-picker/a/thumb.ts",
            "src/features/editor/legacy-paint/paint.ts",
            "src/shared/http/client.ts",
        ] {
            f.module(rel, None);
        }
        for (rel, role) in [
            ("src/features/editor/_shared", DirRole::PrivateZone),
            ("src/features/editor/ui", DirRole::Segment { entry: None }),
        ] {
            f.g.dir_roles
                .insert(PathBuf::from(format!("{ROOT}/{rel}")), role);
        }
        f
    }

    fn check(f: &Fixture, options: serde_json::Value) -> Vec<Violation> {
        let options: UnclaimedFolderOptions = serde_json::from_value(options).unwrap();
        UnclaimedFolder.check(&f.g, ProjectScope, &options, Severity::Error)
    }

    fn files(found: &[Violation]) -> Vec<String> {
        found
            .iter()
            .map(|v| v.file.to_string_lossy().replace('\\', "/"))
            .collect()
    }

    #[test]
    fn a_folder_no_layer_claims_is_reported_on_its_first_module_with_all_its_modules() {
        let f = editor();
        let found = check(&f, serde_json::json!({ "layers": ["editor-root"] }));
        assert_eq!(
            files(&found),
            [
                format!("{ROOT}/src/features/editor/image-picker/a/thumb.ts"),
                format!("{ROOT}/src/features/editor/legacy-paint/paint.ts"),
            ]
        );
        assert_eq!(
            found[0].message,
            "`src/features/editor/image-picker` belongs to no layer of its own and falls into `editor-root`, which may import 3 layers"
        );
        assert_eq!(found[0].modules.len(), 2);
        assert_eq!(found[0].location.line, 1);
    }

    #[test]
    fn allow_and_unlisted_layers_report_nothing() {
        let f = editor();
        let found = check(
            &f,
            serde_json::json!({ "layers": ["editor-root"], "allow": ["src/features/editor/legacy-*"] }),
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("image-picker"));
        assert!(check(&f, serde_json::json!({ "layers": ["core"] })).is_empty());
        assert!(check(&f, serde_json::json!({})).is_empty());
    }

    #[test]
    fn unknown_option_and_bad_glob_are_rejected() {
        for bad in [
            serde_json::json!({ "layer": ["editor-root"] }),
            serde_json::json!({ "allow": ["src/[x"] }),
        ] {
            assert!(serde_json::from_value::<UnclaimedFolderOptions>(bad).is_err());
        }
    }
}
