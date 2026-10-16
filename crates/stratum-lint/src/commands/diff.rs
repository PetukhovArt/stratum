use std::fmt::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::Config;
use stratum_core::types::ContainerKind;
use stratum_engine::{Drift, SOURCE_EXTENSIONS, paths};

use crate::commands::lint;
use crate::git;

const MAX_ITEMS: usize = 10;
const MAX_BYTES: usize = 8 * 1024;
const MAX_CYCLE_MEMBERS: usize = 5;

pub fn run(root: &Utf8Path, base: &str, config_path: Option<&Utf8Path>) -> miette::Result<i32> {
    let config = lint::load_config(root, config_path)?.config;
    let sha = git::short_sha(root, base)?;
    let tree = git::tree_at(root, base, &pathspecs(&config))?;
    let _link = HeadNodeModulesInBase::create(root, tree.path());
    let drift =
        stratum_engine::drift(tree.path(), root, &config).map_err(|e| miette::miette!("{e}"))?;
    let prefix = git::prefix(root).unwrap_or_default();
    print!(
        "{}",
        markdown(&drift, &sha, prefix.trim_end_matches('/'), base)
    );
    Ok(i32::from(!drift.is_empty()))
}

fn pathspecs(config: &Config) -> Vec<String> {
    let layers: Vec<&Utf8Path> = config.layers.iter().map(|l| l.path.as_path()).collect();
    let outermost = layers
        .iter()
        .filter(|p| !layers.iter().any(|o| o != *p && p.starts_with(o)));
    let mut specs: Vec<String> = Vec::new();
    for path in outermost {
        if stratum_engine::is_source(path) {
            specs.push(path.to_string());
        }
        specs.extend(
            SOURCE_EXTENSIONS
                .iter()
                .map(|ext| format!("{path}/*.{ext}")),
        );
    }
    specs.push("tsconfig*.json".into());
    specs
}

struct HeadNodeModulesInBase(Option<Utf8PathBuf>);

impl HeadNodeModulesInBase {
    fn create(root: &Utf8Path, tree: &Utf8Path) -> Self {
        let target = paths::canonicalize(&root.join("node_modules")).ok();
        let Some(target) = target.filter(|t| t.is_dir() && extends_anything(tree)) else {
            return Self(None);
        };
        let link = tree.join("node_modules");
        match symlink_dir(&target, &link) {
            Ok(()) => Self(Some(link)),
            Err(e) => {
                eprintln!(
                    "warning: could not link {target} into the base tree ({e}); a tsconfig `extends` into node_modules may not resolve at the base"
                );
                Self(None)
            }
        }
    }
}

impl Drop for HeadNodeModulesInBase {
    fn drop(&mut self) {
        if let Some(link) = &self.0 {
            let _ = remove_link(link);
        }
    }
}

fn extends_anything(tree: &Utf8Path) -> bool {
    let Ok(entries) = tree.read_dir_utf8() else {
        return false;
    };
    entries.filter_map(Result::ok).any(|e| {
        e.file_name().starts_with("tsconfig")
            && e.path().extension() == Some("json")
            && std::fs::read_to_string(e.path()).is_ok_and(|t| t.contains("\"extends\""))
    })
}

#[cfg(unix)]
fn symlink_dir(target: &Utf8Path, link: &Utf8Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(unix)]
fn remove_link(link: &Utf8Path) -> std::io::Result<()> {
    std::fs::remove_file(link)
}

#[cfg(windows)]
fn symlink_dir(target: &Utf8Path, link: &Utf8Path) -> std::io::Result<()> {
    if std::os::windows::fs::symlink_dir(target, link).is_ok() {
        return Ok(());
    }
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J", link.as_str(), target.as_str()])
        .stdout(std::process::Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("mklink /J failed"))
    }
}

#[cfg(windows)]
fn remove_link(link: &Utf8Path) -> std::io::Result<()> {
    std::fs::remove_dir(link)
}

fn markdown(drift: &Drift, sha: &str, prefix: &str, base: &str) -> String {
    let mut lines = vec![
        if prefix.is_empty() {
            "<!-- stratum:drift -->".to_owned()
        } else {
            format!("<!-- stratum:drift {prefix} -->")
        },
        format!("### Stratum: architecture drift vs {sha}"),
        summary(drift),
    ];
    section(
        &mut lines,
        "Layer edges grown",
        drift.layer_edges.iter().map(|e| {
            let undeclared = if e.declared {
                ""
            } else {
                " (not in `depends_on`)"
            };
            format!(
                "- `{}` -> `{}`: {} -> {} imports{undeclared}",
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

fn summary(drift: &Drift) -> String {
    if drift.is_empty() {
        return "No architecture drift.".into();
    }
    let counts = [
        (
            drift.layer_edges.len(),
            "grown layer edge",
            "grown layer edges",
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
