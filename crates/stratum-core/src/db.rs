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

/// Opaque snapshot of the Compound DAG. Real shape lives in `stratum-graph` (Phase 2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompoundGraphSnapshot {
    pub module_count: usize,
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
}
