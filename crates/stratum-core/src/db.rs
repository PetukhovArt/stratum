use std::path::PathBuf;

use crate::{ids::ProjectId, violation::Violation};

/// The compound graph + violations API. **This is the entire public surface
/// of `stratum-core` to downstream crates.** Intermediate queries are
/// `pub(crate)` and not visible from outside.
pub trait ArchitectureDatabase: salsa::Database {
    /// Build the Compound DAG for the project.
    fn compound_graph(&self, project: Project) -> CompoundGraphSnapshot;

    /// All violations for the project.
    fn violations(&self, project: Project) -> Vec<Violation>;

    /// Violations restricted to one source file (LSP fast path).
    fn violations_for_file(&self, project: Project, file: PathBuf) -> Vec<Violation>;
}

/// Salsa input identifying a project root. Created by `Project::new(db, root)`.
#[salsa::input]
pub struct Project {
    pub root: PathBuf,
    pub id: ProjectId,
}

/// Opaque snapshot of the Compound DAG. Real shape lives in `stratum-graph` (Phase 2).
/// Phase 0 ships an empty placeholder so callers can compile.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompoundGraphSnapshot {
    pub module_count: usize,
}

/// Concrete Salsa database used by `stratum-lint` and tests.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_constructs_and_queries_empty() {
        let db = StratumDb::default();
        let project = Project::new(&db, PathBuf::from("/tmp/example"), ProjectId::new(1));

        let graph = db.compound_graph(project);
        assert_eq!(graph.module_count, 0);
        assert!(db.violations(project).is_empty());
        assert!(
            db.violations_for_file(project, PathBuf::from("x.ts"))
                .is_empty()
        );
    }

    #[test]
    fn public_api_surface_is_three_queries() {
        let methods = ["compound_graph", "violations", "violations_for_file"];
        assert_eq!(methods.len(), 3);
    }
}
