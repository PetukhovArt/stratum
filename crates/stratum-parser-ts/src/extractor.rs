use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use stratum_core::{edge::EdgeKind, purity::Purity};

use crate::source_span::SourceSpan;

/// A raw import edge as seen by the parser, before assignment of `ModuleId`.
/// The parser does not know about `ModuleId` — that's `stratum-graph`'s job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawImport {
    /// The literal specifier as written in source: `'./foo'`, `'@/lib/bar'`, etc.
    pub specifier: String,
    /// The byte range of the specifier in source.
    pub span: SourceSpan,
    /// Static `import`/`export from`, dynamic `import()`, or something else.
    pub kind: EdgeKind,
    /// True if the import is type-only (`import type {...}`). Type-only imports
    /// are still recorded so visibility rules can run on them; rules that care
    /// about runtime edges can filter on this.
    pub type_only: bool,
}

/// Everything the parser knows about one source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedData {
    pub source_path: Utf8PathBuf,
    pub imports: Vec<RawImport>,
    /// Parser-level errors that did not prevent extraction (e.g. recovered
    /// syntax errors).
    pub diagnostics: Vec<ParserDiagnostic>,
    /// `@stratum-stage N` annotation (Phase 5). `None` if absent.
    #[serde(default)]
    pub stage: Option<Purity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParserDiagnostic {
    pub message: String,
    pub span: SourceSpan,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("Unsupported file extension for path {0}")]
    UnsupportedExtension(Utf8PathBuf),
    #[error("Parser produced a hard error in {path}: {message}")]
    Hard { path: Utf8PathBuf, message: String },
}

/// Language adapter contract — one impl per language.
pub trait LanguageExtractor: Send + Sync {
    /// Return true if this extractor handles the given path (by extension).
    fn handles(&self, path: &Utf8Path) -> bool;

    /// Parse `source` from `path` and produce extracted data.
    ///
    /// # Errors
    /// Returns `ExtractError::UnsupportedExtension` if the file extension is not
    /// recognized by this extractor, or `ExtractError::Hard` if parsing fails
    /// unrecoverably.
    fn extract(&self, path: &Utf8Path, source: &str) -> Result<ExtractedData, ExtractError>;
}
