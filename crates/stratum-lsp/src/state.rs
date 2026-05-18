use std::collections::HashMap;
use std::sync::Arc;

use camino::Utf8PathBuf;
use stratum_config::Config;
use stratum_core::violation::Violation;
use stratum_lint::engine::EngineInput;
use stratum_lint::pipeline;
use stratum_lint::zero_config;
use tower_lsp::lsp_types::{Diagnostic, InitializeParams, Url};

use crate::diagnostics;

#[derive(Debug, Default)]
pub struct ServerState {
    pub project_root: Option<Utf8PathBuf>,
    pub config: Option<Arc<Config>>,
    pub input: Option<EngineInput>,
    pub doc_text: HashMap<Url, String>,
    pub last_diagnostics: HashMap<Url, Vec<Diagnostic>>,
    pub last_violations: HashMap<Url, Vec<Violation>>,
}

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("no workspace folder or root_uri provided in initialize")]
    NoRoot,
    #[error(transparent)]
    Pipeline(#[from] pipeline::PipelineError),
    #[error("uri is not a file path: {0}")]
    #[allow(dead_code)]
    NonFileUri(Url),
}

impl ServerState {
    pub fn init(&mut self, params: &InitializeParams) -> Result<(), StateError> {
        let root = resolve_root(params).ok_or(StateError::NoRoot)?;
        let config = zero_config::infer(&root);
        let arc_config = Arc::new(config.clone());
        let input = pipeline::build(&root, &config)?;
        self.project_root = Some(root);
        self.config = Some(arc_config);
        self.input = Some(input);
        Ok(())
    }

    pub fn rebuild(&mut self) -> Result<(), StateError> {
        let Some(root) = self.project_root.clone() else {
            return Err(StateError::NoRoot);
        };
        let Some(config) = self.config.clone() else {
            return Err(StateError::NoRoot);
        };
        let input = pipeline::build(&root, &config)?;
        self.input = Some(input);
        Ok(())
    }

    #[must_use]
    pub fn compute_diagnostics_for(&self, uri: &Url) -> (Vec<Diagnostic>, Vec<Violation>) {
        let Some(input) = self.input.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let Ok(path) = uri.to_file_path() else {
            return (Vec::new(), Vec::new());
        };
        let all = stratum_lint::engine::RuleEngine::run(input);
        let mine: Vec<Violation> = all.into_iter().filter(|v| v.file == path).collect();
        let diags = mine.iter().map(diagnostics::convert).collect();
        (diags, mine)
    }

    #[must_use]
    pub fn last_suggestion_for(&self, uri: &Url, line: u32) -> Option<&str> {
        self.last_violations
            .get(uri)?
            .iter()
            .find(|v| v.location.line.saturating_sub(1) == line)
            .and_then(|v| v.suggestion.as_deref())
    }
}

fn resolve_root(params: &InitializeParams) -> Option<Utf8PathBuf> {
    if let Some(folders) = &params.workspace_folders {
        if let Some(first) = folders.first() {
            if let Ok(p) = first.uri.to_file_path() {
                return Utf8PathBuf::from_path_buf(p).ok();
            }
        }
    }
    #[allow(deprecated)]
    if let Some(uri) = &params.root_uri {
        if let Ok(p) = uri.to_file_path() {
            return Utf8PathBuf::from_path_buf(p).ok();
        }
    }
    None
}
