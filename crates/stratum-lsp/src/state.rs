use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use camino::Utf8PathBuf;
use tower_lsp::lsp_types::{Diagnostic, InitializeParams, Url};

use crate::diagnostics::DiagnosticsByFile;
use crate::paths::uri_to_path;

#[derive(Debug, Default)]
pub struct ServerState {
    pub root: Option<Utf8PathBuf>,
    pub open: HashMap<String, Url>,
    pub diagnostics: Option<DiagnosticsByFile>,
    pub analysis_started: Option<SystemTime>,
    pub last_error: Option<String>,
}

impl ServerState {
    #[must_use]
    pub fn diagnostics_for(&self, key: &str) -> Vec<Diagnostic> {
        self.diagnostics
            .as_ref()
            .and_then(|d| d.get(key))
            .map(|f| f.items.clone())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn is_stale(&self, path: &Path) -> bool {
        let Some(started) = self.analysis_started else {
            return true;
        };
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map_or(true, |modified| modified >= started)
    }
}

#[must_use]
pub fn resolve_root(params: &InitializeParams) -> Option<Utf8PathBuf> {
    let from_folders = params
        .workspace_folders
        .as_ref()
        .and_then(|f| f.first())
        .and_then(|f| uri_to_path(&f.uri));
    #[allow(deprecated)]
    let from_root_uri = params.root_uri.as_ref().and_then(uri_to_path);
    #[allow(deprecated)]
    let from_root_path = params.root_path.as_ref().map(std::path::PathBuf::from);
    let path = from_folders
        .or(from_root_uri)
        .or(from_root_path)
        .or_else(|| std::env::current_dir().ok())?;
    Utf8PathBuf::from_path_buf(path).ok()
}
