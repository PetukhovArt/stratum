use std::sync::Arc;

use camino::Utf8PathBuf;
use stratum_core::{severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

/// Input handed to the engine: the materialised graph + the resolved config +
/// the project root used to derive relative paths for override matching.
#[derive(Debug, Clone)]
pub struct EngineInput {
    pub graph: Arc<CompoundGraph>,
    pub config: Arc<stratum_config::Config>,
    pub project_root: Utf8PathBuf,
}

/// Single-shot engine over `stratum_rules::run_all`.
///
/// Phase 4 ships this as a plain wrapper around the Phase 3 orchestrator. The
/// Salsa per-rule `#[salsa::tracked]` wiring sketched in the plan is deferred
/// until Phase 7 (LSP), where incremental recompute matters most. The current
/// shape already keeps the API stable: callers depend on `RuleEngine::run` /
/// `run_for_file`, not on whether the inner cache is `HashMap` or Salsa.
#[derive(Debug)]
pub struct RuleEngine;

impl RuleEngine {
    /// Run every registered rule once; honour per-file severity overrides.
    /// Returns violations sorted by `(rule_id, first_module_id)`.
    #[must_use]
    pub fn run(input: &EngineInput) -> Vec<Violation> {
        stratum_rules::run_all(&input.graph, &input.config, &input.project_root)
            .unwrap_or_default()
    }

    /// Filter violations to those whose `file` matches `file`. Mirrors
    /// `ArchitectureDatabase::violations_for_file`.
    #[must_use]
    pub fn run_for_file(input: &EngineInput, file: &std::path::Path) -> Vec<Violation> {
        Self::run(input)
            .into_iter()
            .filter(|v| v.file == file)
            .collect()
    }

    #[must_use]
    pub fn worst_severity(violations: &[Violation]) -> Severity {
        violations
            .iter()
            .map(|v| v.severity)
            .max()
            .unwrap_or(Severity::Off)
    }
}
