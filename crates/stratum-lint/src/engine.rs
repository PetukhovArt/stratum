use std::sync::Arc;

use camino::Utf8PathBuf;
use stratum_core::{severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;
use stratum_rules::RuleRegistry;

#[derive(Debug)]
pub struct EngineInput {
    pub graph: Arc<CompoundGraph>,
    pub config: Arc<stratum_config::Config>,
    pub project_root: Utf8PathBuf,
    pub registry: Arc<RuleRegistry>,
}

#[derive(Debug)]
pub struct RuleEngine;

impl RuleEngine {
    /// Run every registered rule once; honour per-file severity overrides.
    /// Returns violations sorted by `(rule_id, first_module_id)`.
    #[must_use]
    pub fn run(input: &EngineInput) -> Vec<Violation> {
        stratum_rules::run_all_with_registry(
            &input.graph,
            &input.config,
            &input.project_root,
            &input.registry,
        )
        .unwrap_or_default()
    }

    /// Filter violations to those whose `file` matches `file`.
    #[allow(dead_code)]
    #[must_use]
    pub fn run_for_file(input: &EngineInput, file: &std::path::Path) -> Vec<Violation> {
        Self::run(input)
            .into_iter()
            .filter(|v| v.file == file)
            .collect()
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn worst_severity(violations: &[Violation]) -> Severity {
        violations
            .iter()
            .map(|v| v.severity)
            .max()
            .unwrap_or(Severity::Off)
    }
}
