use std::collections::BTreeMap;
use std::path::Path;

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, DirRole};

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

pub(super) fn entry_file_phrase(graph: &CompoundGraph) -> String {
    let names = graph
        .containers
        .values()
        .filter_map(|c| graph.modules.get(&c.entry?)?.path.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    most_common(names).map_or_else(|| "an entry file".into(), |n| format!("'{n}'"))
}

pub(super) fn private_zone_phrase(graph: &CompoundGraph) -> String {
    let names = graph
        .dir_roles
        .iter()
        .filter(|(_, role)| **role == DirRole::PrivateZone)
        .filter_map(|(dir, _)| dir.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    most_common(names).map_or_else(|| "a private-zone folder".into(), |n| format!("'{n}/'"))
}

fn most_common(names: impl Iterator<Item = String>) -> Option<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for name in names {
        *counts.entry(name).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(name, _)| name)
}
