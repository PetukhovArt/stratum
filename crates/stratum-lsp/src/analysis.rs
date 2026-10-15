use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use camino::Utf8Path;
use stratum_config::ConfigError;
use stratum_core::ids::ModuleId;
use stratum_graph::{BuildError, GraphBuilder};
use stratum_lint::engine::{EngineInput, RuleEngine};
use stratum_lint::pipeline::{self, PipelineError};
use stratum_lint::zero_config;

use crate::diagnostics::{self, DiagnosticsByFile};
use crate::paths::path_key;

pub const CONFIG_FILE: &str = "stratum.config.jsonc";
pub const TSCONFIG_FILE: &str = "tsconfig.json";

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Pipeline(#[from] PipelineError),
}

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
    modules: HashMap<String, ModuleId>,
    built: SystemTime,
}

#[derive(Debug)]
pub struct Loaded {
    pub inputs: Vec<PathBuf>,
    pub project: Result<Project, AnalysisError>,
}

impl Project {
    #[must_use]
    pub fn load(root: &Utf8Path) -> Loaded {
        let built = SystemTime::now();
        let config_path = root.join(CONFIG_FILE);
        let mut inputs = vec![
            config_path.clone().into_std_path_buf(),
            root.join(TSCONFIG_FILE).into_std_path_buf(),
        ];
        let config = if config_path.is_file() {
            match stratum_config::parse_file(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    return Loaded {
                        inputs,
                        project: Err(e.into()),
                    };
                }
            }
        } else {
            zero_config::infer(root)
        };
        inputs.extend(
            config
                .rules
                .values()
                .filter_map(|r| r.script.as_ref())
                .map(|s| root.join(s).into_std_path_buf()),
        );
        let project = pipeline::build_with_builder(root, &config)
            .map(|(input, builder)| {
                let root = input.project_root.as_std_path();
                let modules = input
                    .graph
                    .modules
                    .values()
                    .map(|m| (path_key(&root.join(&m.path)), m.id))
                    .collect();
                Self {
                    builder,
                    input,
                    modules,
                    built,
                }
            })
            .map_err(AnalysisError::from);
        Loaded { inputs, project }
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

    #[must_use]
    pub fn diagnostics(&self) -> DiagnosticsByFile {
        diagnostics::group(&self.input, &RuleEngine::run(&self.input))
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
    pub outcome: Result<Option<DiagnosticsByFile>, AnalysisError>,
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
                    outcome: Ok(changed.then(|| p.diagnostics())),
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
                outcome: Ok(Some(project.diagnostics())),
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
        let before = project.diagnostics();
        assert_eq!(
            project.apply(cart.as_std_path(), &text).unwrap(),
            Applied::Changed
        );
        let incremental = project.diagnostics();

        std::fs::write(&cart, &text).unwrap();
        let full = Project::load(&root).project.unwrap().diagnostics();

        assert_ne!(incremental, before);
        assert_eq!(incremental, full);
    }
}
