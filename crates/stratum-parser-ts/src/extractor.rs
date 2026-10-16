use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use stratum_core::{
    edge::EdgeKind,
    purity::Purity,
    types::{Bindings, ExportOrigin},
};

use crate::source_span::SourceSpan;

/// A raw import edge as seen by the parser, before assignment of `ModuleId`.
/// The parser does not know about `ModuleId` — that's `stratum-graph`'s job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawImport {
    /// The literal specifier as written in source: `'./foo'`, `'@/lib/bar'`, etc.
    pub specifier: String,
    /// The byte range of the specifier in source.
    pub span: SourceSpan,
    /// The whole import statement, or the `import()` / `require()` / `import.meta.glob()` call.
    pub statement: SourceSpan,
    /// Static `import`/`export from`, dynamic `import()`, or something else.
    pub kind: EdgeKind,
    /// True if the import is type-only (`import type {...}`). Type-only imports
    /// are still recorded so visibility rules can run on them; rules that care
    /// about runtime edges can filter on this.
    pub type_only: bool,
    pub bindings: Bindings,
    /// `specifier` is an `import.meta.glob` pattern, not a path.
    pub glob: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCall {
    /// The entry as written: `inject`, `obj.get`, `*.get`.
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExportDecl {
    pub name: String,
    pub type_only: bool,
    pub span: SourceSpan,
    /// `Reexport::import` indexes [`ExtractedData::imports`].
    pub origin: ExportOrigin,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExports {
    pub decls: Vec<RawExportDecl>,
    /// Indices into [`ExtractedData::imports`] of every `export * from`.
    pub stars: Vec<usize>,
    pub unimportable: u32,
}

/// A `stratum-ignore` comment; `span` covers the comment with its delimiters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSuppression {
    pub span: SourceSpan,
    pub slugs: Vec<String>,
    pub reason: Option<String>,
}

/// Everything the parser knows about one source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedData {
    pub imports: Vec<RawImport>,
    /// `@stratum-purity N` annotation (Phase 5). `None` if absent.
    #[serde(default)]
    pub purity: Option<Purity>,
    #[serde(default)]
    pub exports: RawExports,
    #[serde(default)]
    pub calls: Vec<RawCall>,
    #[serde(default)]
    pub suppressions: Vec<RawSuppression>,
}

/// The calls the parser records. Entry syntax: `name` is a plain call of `name`, of a local
/// alias of the named import `name`, or `ns.name()` on a namespace import `ns`; `obj.name` is a
/// member call on the identifier `obj`; `*.name` is a member call on anything.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackedCalls {
    entries: Vec<TrackedCall>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct TrackedCall {
    pub(crate) entry: String,
    pub(crate) pattern: CallPattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum CallPattern {
    Plain(String),
    Member { object: String, name: String },
    AnyMember(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Invalid call entry {0:?}: expected `name`, `obj.name` or `*.name`")]
pub struct InvalidCallEntry(pub String);

impl TrackedCalls {
    /// # Errors
    /// [`InvalidCallEntry`] if `entry` is not `name`, `obj.name` or `*.name` over identifiers.
    pub fn insert(&mut self, entry: &str) -> Result<(), InvalidCallEntry> {
        let pattern = CallPattern::parse(entry).ok_or_else(|| InvalidCallEntry(entry.into()))?;
        if !self.entries.iter().any(|e| e.entry == entry) {
            self.entries.push(TrackedCall {
                entry: entry.to_owned(),
                pattern,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn entries(&self) -> &[TrackedCall] {
        &self.entries
    }
}

impl CallPattern {
    fn parse(entry: &str) -> Option<Self> {
        let Some((object, name)) = entry.split_once('.') else {
            return is_identifier(entry).then(|| Self::Plain(entry.to_owned()));
        };
        if !is_identifier(name) {
            return None;
        }
        if object == "*" {
            return Some(Self::AnyMember(name.to_owned()));
        }
        is_identifier(object).then(|| Self::Member {
            object: object.to_owned(),
            name: name.to_owned(),
        })
    }
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
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
