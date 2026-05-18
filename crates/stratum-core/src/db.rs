use std::path::PathBuf;

use crate::{ids::ProjectId, violation::Violation};

/// The compound graph + violations API. The entire public surface of
/// `stratum-core` to downstream crates — intermediate queries are
/// `pub(crate)` (PRD decision D6).
pub trait ArchitectureDatabase: salsa::Database {
    fn compound_graph(&self, project: Project) -> CompoundGraphSnapshot;
    fn violations(&self, project: Project) -> Vec<Violation>;
    fn violations_for_file(&self, project: Project, file: PathBuf) -> Vec<Violation>;
}

#[salsa::input]
pub struct Project {
    pub root: PathBuf,
    pub id: ProjectId,
}

/// Opaque marker carrying a serialised snapshot of the Compound DAG.
///
/// The typed shape lives in `stratum-graph::snapshot::GraphSnapshot`.
/// `stratum-core` cannot depend on `stratum-graph` (it would form a cycle),
/// so this trait return-value smuggles the materialised graph as a JSON
/// string. Callers that need the typed graph go through `stratum-graph`
/// directly on the materialised `CompoundGraph` via the CLI / LSP layer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompoundGraphSnapshot {
    pub json: String,
}

#[salsa::db]
#[derive(Default, Clone)]
pub struct StratumDb {
    storage: salsa::Storage<Self>,
}

impl std::fmt::Debug for StratumDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StratumDb").finish_non_exhaustive()
    }
}

#[salsa::db]
impl salsa::Database for StratumDb {
    fn salsa_event(&self, _event: &dyn Fn() -> salsa::Event) {}
}

impl ArchitectureDatabase for StratumDb {
    fn compound_graph(&self, _project: Project) -> CompoundGraphSnapshot {
        CompoundGraphSnapshot::default()
    }

    fn violations(&self, _project: Project) -> Vec<Violation> {
        Vec::new()
    }

    fn violations_for_file(&self, _project: Project, _file: PathBuf) -> Vec<Violation> {
        Vec::new()
    }
}

/// Intermediate Salsa input: a single source file's content.
///
/// Crate-private — only `stratum-graph` (Phase 2) and `stratum-lint` (Phase 4)
/// inside this workspace read/write it. External consumers see only the public
/// queries on [`ArchitectureDatabase`].
#[salsa::input]
pub(crate) struct SourceFile {
    pub(crate) path: camino::Utf8PathBuf,
    #[return_ref]
    pub(crate) text: String,
}

/// Intermediate Salsa input: extracted module data for one file.
///
/// The actual extraction is performed by a `LanguageExtractor` implementation
/// in `stratum-parser-ts` / `stratum-parser-vue`; here we just memoize the
/// result keyed by [`SourceFile`].
///
/// The body uses [`crate::violation::SourceLocation`] instead of `RawImport`
/// to avoid a `stratum-core → stratum-parser-ts` dep cycle — the orchestration
/// crate translates between them before setting this input.
#[salsa::input]
pub(crate) struct ExtractedFile {
    pub(crate) source: SourceFile,
    #[return_ref]
    pub(crate) raw_imports: Vec<crate::violation::SourceLocation>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn db_constructs_and_queries_empty() {
        let db = StratumDb::default();
        let project = Project::new(&db, PathBuf::from("/tmp/example"), ProjectId::new(1));

        let graph = db.compound_graph(project);
        assert!(graph.json.is_empty());
        assert!(db.violations(project).is_empty());
        assert!(
            db.violations_for_file(project, PathBuf::from("x.ts"))
                .is_empty()
        );
    }

    #[test]
    fn source_file_and_extracted_file_construct() {
        let db = StratumDb::default();
        let sf = SourceFile::new(
            &db,
            camino::Utf8PathBuf::from("src/foo.ts"),
            "export const x = 1;\n".to_string(),
        );
        assert_eq!(sf.text(&db), "export const x = 1;\n");

        let ef = ExtractedFile::new(&db, sf, Vec::new());
        assert_eq!(ef.source(&db), sf);
        assert!(ef.raw_imports(&db).is_empty());
    }
}
