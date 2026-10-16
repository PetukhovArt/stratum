use std::path::Path;

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    types::{Container, ContainerKind},
    violation::Violation,
};
use stratum_graph::{CompoundGraph, DirRole};

use super::folder::at_folder;
use crate::ids::NAMING;
use crate::options::{FolderCase, NamingOptions};
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct Naming;

impl Rule for Naming {
    type Scope = ProjectScope;
    type Options = NamingOptions;

    fn id(&self) -> RuleId {
        NAMING
    }
    fn slug(&self) -> &'static str {
        "stratum/naming"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        options: &NamingOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let mut out = Vec::new();
        for (dir, role) in &graph.dir_roles {
            if *role == DirRole::LayerRoot {
                continue;
            }
            let Some(name) = dir.file_name().map(|n| n.to_string_lossy().into_owned()) else {
                continue;
            };
            let entry = match role {
                DirRole::Container(id) => graph.containers.get(id).and_then(|c| c.entry),
                _ => None,
            };
            if options.folder_case == Some(FolderCase::Kebab)
                && *role != DirRole::PrivateZone
                && !is_kebab(&name)
            {
                out.extend(finding(
                    graph,
                    severity,
                    dir,
                    entry,
                    format!("Folder '{name}' is not kebab-case"),
                    format!("Rename '{name}' to '{}'.", to_kebab(&name)),
                ));
            }
            if options.reserved.contains(&name) {
                out.extend(finding(
                    graph,
                    severity,
                    dir,
                    entry,
                    format!("Folder '{name}' has a reserved name"),
                    format!(
                        "Name '{name}' after what it holds, not its kind, or move its files next to the code that uses them."
                    ),
                ));
            }
        }
        if options.duplicate_slices {
            out.extend(duplicate_slices(graph, severity));
        }
        out
    }
}

fn finding(
    graph: &CompoundGraph,
    severity: Severity,
    dir: &Path,
    entry: Option<ModuleId>,
    message: String,
    fix: String,
) -> Option<Violation> {
    at_folder(graph, NAMING, severity, dir, entry, message, fix)
}

fn is_kebab(name: &str) -> bool {
    !name.is_empty()
        && name.split('-').all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

fn to_kebab(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            let starts_word = c.is_ascii_uppercase()
                && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
            if starts_word {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        previous = Some(c);
    }
    out.trim_end_matches('-').to_owned()
}

fn slice_name(slice: &Container) -> String {
    slice
        .name
        .split('.')
        .next()
        .unwrap_or(&slice.name)
        .to_ascii_lowercase()
}

fn same_concept(a: &str, b: &str) -> bool {
    a == b || a.strip_suffix('s') == Some(b) || b.strip_suffix('s') == Some(a)
}

fn duplicate_slices(graph: &CompoundGraph, severity: Severity) -> Vec<Violation> {
    let mut slices: Vec<(&Container, String)> = graph
        .containers
        .values()
        .filter(|c| c.kind == ContainerKind::Slice)
        .map(|c| (c, slice_name(c)))
        .collect();
    slices.sort_by(|(a, _), (b, _)| a.path.cmp(&b.path));
    let layer_of = |c: &Container| {
        graph
            .layers
            .get(&c.layer)
            .map_or("?", |l| l.name.as_str())
            .to_owned()
    };
    slices
        .iter()
        .enumerate()
        .filter_map(|(i, (slice, name))| {
            let twin = slices[..i]
                .iter()
                .find(|(_, other)| same_concept(name, other))?
                .0;
            at_folder(
                graph,
                NAMING,
                severity,
                &slice.path,
                slice.entry,
                format!(
                    "Slice '{}' in layer '{}' repeats the name of slice '{}' in layer '{}'",
                    slice.name,
                    layer_of(slice),
                    twin.name,
                    layer_of(twin),
                ),
                format!(
                    "Merge '{}' and '{}' into one slice, or rename one so the two names mean different things.",
                    slice.name, twin.name
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_case_rejects_every_other_case() {
        for ok in ["cart", "order-line", "v2", "2fa-codes"] {
            assert!(is_kebab(ok), "{ok}");
        }
        for bad in [
            "",
            "Cart",
            "orderLine",
            "order_line",
            "order--line",
            "-order",
            "order-",
            "a.b",
        ] {
            assert!(!is_kebab(bad), "{bad}");
        }
    }

    #[test]
    fn the_suggested_name_is_kebab_case() {
        for (from, to) in [
            ("OrderLine", "order-line"),
            ("orderLine", "order-line"),
            ("order_line", "order-line"),
            ("Order Line", "order-line"),
            ("_shared-ui", "shared-ui"),
        ] {
            assert_eq!(to_kebab(from), to);
            assert!(is_kebab(&to_kebab(from)));
        }
    }

    #[test]
    fn names_differing_by_a_plural_s_are_one_concept() {
        assert!(same_concept("user", "users"));
        assert!(same_concept("users", "user"));
        assert!(same_concept("user", "user"));
        assert!(!same_concept("user", "usage"));
    }
}
