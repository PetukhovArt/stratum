use std::path::Path;

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

pub(super) fn at_module(
    graph: &CompoundGraph,
    rule: RuleId,
    severity: Severity,
    at: ModuleId,
    message: String,
    fix: String,
) -> Option<Violation> {
    let file = graph.modules.get(&at)?.path.clone();
    Some(Violation {
        rule,
        severity,
        message,
        file,
        location: SourceLocation { line: 1, column: 1 },
        modules: vec![at],
        edge: None,
        fix: Some(fix),
    })
}

pub(super) fn at_folder(
    graph: &CompoundGraph,
    rule: RuleId,
    severity: Severity,
    dir: &Path,
    entry: Option<ModuleId>,
    message: String,
    fix: String,
) -> Option<Violation> {
    let at = entry.or_else(|| graph.first_module_in(dir))?;
    at_module(graph, rule, severity, at, message, fix)
}

pub(super) fn relative(dir: &Path, base: &Path) -> String {
    dir.strip_prefix(base)
        .unwrap_or(dir)
        .to_string_lossy()
        .replace('\\', "/")
}
