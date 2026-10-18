use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_core::ids::ModuleId;
use stratum_engine::{
    BASELINE_FILE, Baseline, CONFIG_FILE, EngineError, EngineInput, config, pipeline,
};
use stratum_graph::{BuildError, GraphBuilder};

use crate::diagnostics::{self, DiagnosticsByFile};
use crate::paths::path_key;

pub const TSCONFIG_FILE: &str = "tsconfig.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    Unchanged,
    Changed,
    Untracked,
    CreatedAfterBuild,
}

#[derive(Debug)]
pub struct Project {
    builder: GraphBuilder,
    input: EngineInput,
    baseline: Option<Baseline>,
    modules: HashMap<String, ModuleId>,
    built: SystemTime,
}

#[derive(Debug)]
pub struct Loaded {
    pub inputs: Vec<PathBuf>,
    pub project: Result<Project, EngineError>,
}

impl Project {
    #[must_use]
    pub fn load(root: &Utf8Path) -> Loaded {
        let mut inputs: Vec<Utf8PathBuf> = config::watched_files(&root.join(CONFIG_FILE));
        inputs.extend([root.join(TSCONFIG_FILE), root.join(BASELINE_FILE)]);
        let project = Self::build(root, &mut inputs);
        Loaded {
            inputs: inputs
                .into_iter()
                .map(Utf8PathBuf::into_std_path_buf)
                .collect(),
            project,
        }
    }

    fn build(root: &Utf8Path, inputs: &mut Vec<Utf8PathBuf>) -> Result<Self, EngineError> {
        let built = SystemTime::now();
        let loaded = config::load(root, None)?;
        inputs.extend(
            pipeline::plugin_scripts(root, &loaded.config)
                .into_iter()
                .map(|(_, script)| script),
        );
        let baseline = Baseline::load_if_exists(&root.join(BASELINE_FILE))?;
        let (input, builder) = pipeline::build_with_builder(root, &loaded)?;
        let modules = input
            .graph
            .modules
            .values()
            .map(|m| (path_key(&m.path), m.id))
            .collect();
        Ok(Self {
            builder,
            input,
            baseline,
            modules,
            built,
        })
    }

    #[must_use]
    pub fn tracks(&self, path: &Path) -> bool {
        self.modules.contains_key(&path_key(path))
    }

    pub fn apply(&mut self, path: &Path, text: &str) -> Result<Applied, BuildError> {
        let Some(&id) = self.modules.get(&path_key(path)) else {
            return Ok(if self.created_after_build(path) {
                Applied::CreatedAfterBuild
            } else {
                Applied::Untracked
            });
        };
        let graph = Arc::make_mut(&mut self.input.graph);
        Ok(if self.builder.update_module(graph, id, text)? {
            Applied::Changed
        } else {
            Applied::Unchanged
        })
    }

    pub fn diagnostics(&self) -> Result<DiagnosticsByFile, EngineError> {
        let mut findings = self.input.violations()?;
        if let Some(baseline) = &self.baseline {
            let baselined = baseline.matches(&findings).baselined;
            findings = findings
                .into_iter()
                .zip(baselined)
                .filter_map(|(f, baselined)| (!baselined).then_some(f))
                .collect();
        }
        Ok(diagnostics::group(
            self.input.project_root.as_std_path(),
            &findings,
        ))
    }

    fn created_after_build(&self, path: &Path) -> bool {
        std::fs::metadata(path)
            .and_then(|m| m.created().or_else(|_| m.modified()))
            .is_ok_and(|t| t >= self.built)
    }
}

#[derive(Debug)]
pub struct Run {
    pub project: Option<Project>,
    pub inputs: Option<Vec<PathBuf>>,
    pub outcome: Result<Option<DiagnosticsByFile>, EngineError>,
}

#[derive(Debug)]
pub enum Text {
    Buffer(String),
    Disk,
}

#[must_use]
pub fn run(
    project: Option<Project>,
    root: &Utf8Path,
    rebuild: bool,
    edits: &[(PathBuf, Text)],
    open: &[(PathBuf, String)],
) -> Run {
    let previous = match project {
        Some(mut p) if !rebuild => match apply_edits(&mut p, edits) {
            Some(changed) => {
                return Run {
                    outcome: if changed {
                        p.diagnostics().map(Some)
                    } else {
                        Ok(None)
                    },
                    project: Some(p),
                    inputs: None,
                };
            }
            None => Some(p),
        },
        other => other,
    };
    rebuild_and_reopen(previous, root, open)
}

fn apply_edits(project: &mut Project, edits: &[(PathBuf, Text)]) -> Option<bool> {
    let mut changed = false;
    for (path, text) in edits {
        let text = match text {
            Text::Buffer(t) => t.clone(),
            Text::Disk if !project.tracks(path) => continue,
            Text::Disk => match std::fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => {
                    tracing::info!("{} unreadable, rebuilding: {e}", path.display());
                    return None;
                }
            },
        };
        match project.apply(path, &text) {
            Ok(Applied::Changed) => changed = true,
            Ok(Applied::Unchanged | Applied::Untracked) => {}
            Ok(Applied::CreatedAfterBuild) => {
                tracing::info!("{} is new, rebuilding", path.display());
                return None;
            }
            Err(e) => {
                tracing::warn!("incremental update failed, rebuilding: {e}");
                return None;
            }
        }
    }
    Some(changed)
}

fn rebuild_and_reopen(
    previous: Option<Project>,
    root: &Utf8Path,
    open: &[(PathBuf, String)],
) -> Run {
    let loaded = Project::load(root);
    match loaded.project {
        Ok(mut project) => {
            for (path, text) in open {
                if let Err(e) = project.apply(path, text) {
                    tracing::warn!("{}: {e}", path.display());
                }
            }
            Run {
                outcome: project.diagnostics().map(Some),
                project: Some(project),
                inputs: Some(loaded.inputs),
            }
        }
        Err(e) => Run {
            project: previous,
            inputs: Some(loaded.inputs),
            outcome: Err(e),
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn fixture_copy() -> tempfile::TempDir {
        let src =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tiny-ts-violations");
        let dir = tempfile::tempdir().unwrap();
        for entry in walkdir(&src) {
            let rel = entry.strip_prefix(&src).unwrap();
            let dest = dir.path().join(rel);
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            std::fs::copy(&entry, dest).unwrap();
        }
        dir
    }

    fn walkdir(dir: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .flat_map(|e| {
                let path = e.unwrap().path();
                if path.is_dir() {
                    walkdir(&path)
                } else {
                    vec![path]
                }
            })
            .collect()
    }

    #[test]
    fn incremental_edit_matches_full_rebuild() {
        let dir = fixture_copy();
        let root = Utf8Path::from_path(dir.path())
            .unwrap()
            .canonicalize_utf8()
            .unwrap();
        let cart = root.join("src/features/cart/cart.tsx");
        let text = format!(
            "{}import \"@/app/main\";\n",
            std::fs::read_to_string(&cart).unwrap()
        );

        let mut project = Project::load(&root).project.unwrap();
        let before = project.diagnostics().unwrap();
        assert_eq!(
            project.apply(cart.as_std_path(), &text).unwrap(),
            Applied::Changed
        );
        let incremental = project.diagnostics().unwrap();

        std::fs::write(&cart, &text).unwrap();
        let full = Project::load(&root).project.unwrap().diagnostics().unwrap();

        assert_ne!(incremental, before);
        assert_eq!(incremental, full);
    }

    #[test]
    fn bad_override_glob_is_an_error_not_empty_diagnostics() {
        let dir = fixture_copy();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let config = root.join(CONFIG_FILE);
        let text = std::fs::read_to_string(&config)
            .unwrap()
            .replace("src/shared/api/**", "src/[shared");
        std::fs::write(&config, text).unwrap();

        let err = Project::load(root).project.unwrap_err();
        assert!(err.to_string().contains("src/[shared"), "{err}");
    }
}
