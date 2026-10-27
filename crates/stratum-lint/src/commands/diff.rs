use std::fmt::Write as _;

use camino::Utf8Path;
use stratum_config::Config;
use stratum_core::types::ContainerKind;
use stratum_engine::{Drift, co_change};
use stratum_rules::CoChangeMiss;

use crate::commands::lint;
use crate::git;

const MAX_ITEMS: usize = 10;
const MAX_BYTES: usize = 8 * 1024;
const MAX_CYCLE_MEMBERS: usize = 5;

pub fn run(root: &Utf8Path, base: &str, config_path: Option<&Utf8Path>) -> miette::Result<i32> {
    let config = lint::load_config(root, config_path)?.config;
    let report = compute(root, base, &config)?;
    print!("{}", report.markdown());
    Ok(i32::from(
        !report.drift.is_empty() || !report.co_change.is_empty(),
    ))
}

pub(crate) struct DriftReport<'a> {
    pub base: &'a str,
    pub sha: String,
    pub drift: Drift,
    /// `stratum/co-change` policies the change breaks.
    pub co_change: Vec<CoChangeMiss>,
    prefix: String,
}

impl DriftReport<'_> {
    pub fn markdown(&self) -> String {
        markdown(
            &self.drift,
            &self.co_change,
            &self.sha,
            self.prefix.trim_end_matches('/'),
            self.base,
        )
    }
}

pub(crate) fn compute<'a>(
    root: &Utf8Path,
    base: &'a str,
    config: &Config,
) -> miette::Result<DriftReport<'a>> {
    let sha = git::short_sha(root, base)?;
    let base_files = git::files_at(root, base, |rel| {
        let path = Utf8Path::new(rel);
        stratum_engine::is_source(path) || path.extension() == Some("json")
    })?;
    let drift =
        stratum_engine::drift(root, base_files, config).map_err(|e| miette::miette!("{e}"))?;
    let changed = git::changed_files(root, base)?;
    Ok(DriftReport {
        base,
        sha,
        drift,
        co_change: co_change::misses(root, config, &changed),
        prefix: git::prefix(root).unwrap_or_default(),
    })
}

fn markdown(
    drift: &Drift,
    co_changes: &[CoChangeMiss],
    sha: &str,
    prefix: &str,
    base: &str,
) -> String {
    let mut lines = vec![
        if prefix.is_empty() {
            "<!-- stratum:drift -->".to_owned()
        } else {
            format!("<!-- stratum:drift {prefix} -->")
        },
        format!("### Stratum: architecture drift vs {sha}"),
        summary(drift, co_changes.len()),
    ];
    section(
        &mut lines,
        "Files that must change together",
        co_changes
            .iter()
            .map(|miss| format!("- {}", co_change::message(miss))),
    );
    let breaking = drift.breaking_imports();
    section(
        &mut lines,
        "Imports that break rules",
        breaking.iter().map(|e| {
            let rules = e
                .breaks
                .iter()
                .map(|(slug, severity)| format!("`{slug}` ({severity})"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("- `{}` -> `{}`: {rules}", e.from, e.to)
        }),
    );
    section(
        &mut lines,
        "Layer edges not in `depends_on`",
        drift.layer_edges.iter().map(|e| {
            format!(
                "- `{}` -> `{}`: {} -> {} imports",
                e.from, e.to, e.base, e.head
            )
        }),
    );
    section(
        &mut lines,
        "New slice edges",
        drift
            .slice_edges
            .iter()
            .map(|e| format!("- `{}` -> `{}`, e.g. `{}`", e.from, e.to, e.example)),
    );
    section(
        &mut lines,
        "Cycles",
        drift.cycles.iter().map(|c| {
            let shown = c.members.iter().take(MAX_CYCLE_MEMBERS);
            let mut members = shown
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(", ");
            if c.members.len() > MAX_CYCLE_MEMBERS {
                let _ = write!(members, ", +{}", c.members.len() - MAX_CYCLE_MEMBERS);
            }
            match c.grown_from {
                Some(from) => format!("- grew {from} -> {}: {members}", c.members.len()),
                None => format!("- new, {} modules: {members}", c.members.len()),
            }
        }),
    );
    section(
        &mut lines,
        "Stage changes",
        drift.stages.iter().map(|s| {
            let change = match s.base {
                Some(base) => format!("stage {base} -> {}", s.head),
                None => format!("new, stage {}", s.head),
            };
            format!("- `{}` ({}): {change}", s.container, kind_label(s.kind))
        }),
    );
    section(
        &mut lines,
        "Slice fan-in grown",
        drift.fan_in.iter().map(|f| {
            let added = f
                .added
                .iter()
                .map(|a| format!("`{a}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "- `{}`: {} -> {} importing slices, new: {added}",
                f.slice, f.base, f.head
            )
        }),
    );
    capped(&lines, base)
}

fn summary(drift: &Drift, co_changes: usize) -> String {
    if drift.is_empty() && co_changes == 0 {
        return "No architecture drift.".into();
    }
    let counts = [
        (
            co_changes,
            "file set that must change together",
            "file sets that must change together",
        ),
        (
            drift.breaking_imports().len(),
            "import that breaks rules",
            "imports that break rules",
        ),
        (
            drift.layer_edges.len(),
            "layer edge not in depends_on",
            "layer edges not in depends_on",
        ),
        (drift.slice_edges.len(), "new slice edge", "new slice edges"),
        (drift.cycles.len(), "cycle change", "cycle changes"),
        (drift.stages.len(), "stage change", "stage changes"),
        (drift.fan_in.len(), "fan-in growth", "fan-in growths"),
    ];
    counts
        .iter()
        .filter(|(n, ..)| *n > 0)
        .map(|(n, one, many)| format!("{n} {}", if *n == 1 { one } else { many }))
        .collect::<Vec<_>>()
        .join(", ")
}

fn section(lines: &mut Vec<String>, title: &str, items: impl ExactSizeIterator<Item = String>) {
    let total = items.len();
    if total == 0 {
        return;
    }
    lines.push(String::new());
    lines.push(format!("#### {title} ({total})"));
    lines.extend(items.take(MAX_ITEMS));
    if total > MAX_ITEMS {
        lines.push(format!("- ... {} more", total - MAX_ITEMS));
    }
}

fn capped(lines: &[String], base: &str) -> String {
    let tail = format!(
        "\nTruncated at 8 KiB: run `stratum-lint diff --base {base}` locally for the full report.\n"
    );
    let mut out = String::new();
    for line in lines {
        if out.len() + line.len() + 1 + tail.len() > MAX_BYTES {
            out.push_str(&tail);
            return out;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn kind_label(kind: ContainerKind) -> &'static str {
    match kind {
        ContainerKind::Layer => "layer",
        ContainerKind::Slice => "slice",
        ContainerKind::SubUnit => "sub-unit",
        ContainerKind::Nested => "nested",
    }
}
