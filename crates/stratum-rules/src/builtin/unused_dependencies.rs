use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;
use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    types::{Bindings, Import},
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, Manifest, SideRole};

use crate::builtin::import_policy::glob_set;
use crate::ids::UNUSED_DEPENDENCIES;
use crate::rule::Rule;
use crate::scope::ProjectScope;

const SLUG: &str = "stratum/unused-dependencies";

#[derive(Debug, Default, Clone, Copy)]
pub struct UnusedDependencies;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyCheck {
    Unused,
    Unlisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ManifestField {
    #[serde(rename = "dependencies")]
    Dependencies,
    #[serde(rename = "devDependencies")]
    DevDependencies,
    #[serde(rename = "peerDependencies")]
    PeerDependencies,
    #[serde(rename = "optionalDependencies")]
    OptionalDependencies,
}

impl ManifestField {
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Dependencies => "dependencies",
            Self::DevDependencies => "devDependencies",
            Self::PeerDependencies => "peerDependencies",
            Self::OptionalDependencies => "optionalDependencies",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnusedDependenciesOptions {
    #[serde(default = "both_checks")]
    pub check: Vec<DependencyCheck>,
    #[serde(default = "default_fields")]
    pub fields: Vec<ManifestField>,
    #[serde(default, deserialize_with = "globs")]
    pub ignore: Vec<String>,
    /// Globs relative to the project root of files outside the layers whose imports count.
    #[serde(default = "default_entries", deserialize_with = "globs")]
    pub entries: Vec<String>,
    #[serde(default = "yes")]
    pub scripts: bool,
}

pub const DEFAULT_ENTRIES: &str = "*.config.{js,cjs,mjs,ts,cts,mts}";

fn both_checks() -> Vec<DependencyCheck> {
    vec![DependencyCheck::Unused, DependencyCheck::Unlisted]
}

fn default_fields() -> Vec<ManifestField> {
    vec![ManifestField::Dependencies, ManifestField::DevDependencies]
}

fn default_entries() -> Vec<String> {
    vec![DEFAULT_ENTRIES.to_owned()]
}

const fn yes() -> bool {
    true
}

fn globs<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let patterns = Vec::<String>::deserialize(d)?;
    glob_set(&patterns).map_err(serde::de::Error::custom)?;
    Ok(patterns)
}

impl Default for UnusedDependenciesOptions {
    fn default() -> Self {
        Self {
            check: both_checks(),
            fields: default_fields(),
            ignore: Vec::new(),
            entries: default_entries(),
            scripts: true,
        }
    }
}

impl Rule for UnusedDependencies {
    type Scope = ProjectScope;
    type Options = UnusedDependenciesOptions;

    fn id(&self) -> RuleId {
        UNUSED_DEPENDENCIES
    }
    fn slug(&self) -> &'static str {
        SLUG
    }
    fn default_severity(&self) -> Severity {
        Severity::Off
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        options: &UnusedDependenciesOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(manifest) = &graph.manifest else {
            return Vec::new();
        };
        let ignore = glob_set(&options.ignore).unwrap_or_default();
        let skip =
            |package: &str| ignore.is_match(package) || manifest.name.as_deref() == Some(package);
        let fields: Vec<&str> = options.fields.iter().map(|f| f.key()).collect();
        let declared: BTreeMap<&str, (&str, SourceLocation)> = manifest
            .declared
            .iter()
            .filter(|d| fields.contains(&d.field.as_str()))
            .map(|d| (d.package.as_str(), (d.field.as_str(), d.location)))
            .collect();
        let importers = importers(graph);
        let mut out = Vec::new();
        if options.check.contains(&DependencyCheck::Unused) {
            let used = Used::of(&importers, manifest, options.scripts);
            for (&package, &(field, location)) in &declared {
                if skip(package) || used.uses(package) {
                    continue;
                }
                out.push(Violation {
                    rule: self.id(),
                    severity,
                    message: format!(
                        "Package `{package}` is declared in `{field}` but nothing imports it"
                    ),
                    file: manifest.path.clone(),
                    location,
                    modules: vec![],
                    edge: None,
                    fix: Some(format!(
                        "Remove `{package}` from package.json, or add it to the rule's `ignore` if it is used another way (a CLI not in `scripts`, a plugin loaded by name, global types)"
                    )),
                });
            }
        }
        if options.check.contains(&DependencyCheck::Unlisted) {
            out.extend(unlisted(
                self.id(),
                severity,
                &importers,
                &declared,
                &skip,
                &fields,
            ));
        }
        out
    }
}

struct Importer<'g> {
    path: &'g Path,
    module: Option<ModuleId>,
    imports: &'g [Import],
    /// Its packages must be declared: not an ignored file.
    checked: bool,
}

fn importers(graph: &CompoundGraph) -> Vec<Importer<'_>> {
    let modules = graph.modules.values().map(|m| Importer {
        path: &m.path,
        module: Some(m.id),
        imports: &m.imports,
        checked: true,
    });
    let sides = graph.side_files.iter().map(|s| Importer {
        path: &s.path,
        module: None,
        imports: &s.imports,
        checked: s.role != SideRole::Ignored,
    });
    modules.chain(sides).collect()
}

struct Used<'a> {
    packages: BTreeSet<&'a str>,
    builtin: bool,
    words: BTreeSet<&'a str>,
    manifest: &'a Manifest,
}

impl<'a> Used<'a> {
    fn of(importers: &'a [Importer<'a>], manifest: &'a Manifest, scripts: bool) -> Self {
        let mut packages = BTreeSet::new();
        let mut builtin = false;
        for import in importers.iter().flat_map(|i| i.imports) {
            match package_of(import) {
                Some(Package::Builtin(bare)) => {
                    builtin = true;
                    packages.extend(bare);
                }
                Some(Package::Npm(name)) => {
                    packages.insert(name);
                }
                None => {}
            }
        }
        let words = if scripts {
            manifest
                .scripts
                .iter()
                .flat_map(|command| {
                    command.split(|c: char| c.is_whitespace() || "&|;()".contains(c))
                })
                .filter(|w| !w.is_empty())
                .collect()
        } else {
            BTreeSet::new()
        };
        Self {
            packages,
            builtin,
            words,
            manifest,
        }
    }

    fn uses(&self, package: &str) -> bool {
        if self.packages.contains(package) {
            return true;
        }
        if let Some(typed) = package.strip_prefix("@types/") {
            if typed == "node" && self.builtin {
                return true;
            }
            let typed = typed.split_once("__").map_or_else(
                || typed.to_owned(),
                |(scope, name)| format!("@{scope}/{name}"),
            );
            return self.packages.contains(typed.as_str());
        }
        match self.manifest.bins.get(package) {
            Some(bins) => bins.iter().any(|b| self.words.contains(b.as_str())),
            None => self.words.contains(unscoped(package)),
        }
    }
}

fn unscoped(package: &str) -> &str {
    package.rsplit('/').next().unwrap_or(package)
}

fn unlisted(
    rule: RuleId,
    severity: Severity,
    importers: &[Importer<'_>],
    declared: &BTreeMap<&str, (&str, SourceLocation)>,
    skip: &dyn Fn(&str) -> bool,
    fields: &[&str],
) -> Vec<Violation> {
    let mut out = Vec::new();
    for importer in importers.iter().filter(|i| i.checked) {
        let mut seen = BTreeSet::new();
        for import in importer.imports {
            let Some(Package::Npm(package)) = package_of(import) else {
                continue;
            };
            let typed = format!(
                "@types/{}",
                package.trim_start_matches('@').replace('/', "__")
            );
            let listed = declared.contains_key(package)
                || (is_type_only(import) && declared.contains_key(typed.as_str()));
            if listed || skip(package) || !seen.insert(package) {
                continue;
            }
            out.push(Violation {
                rule,
                severity,
                message: format!(
                    "Package `{package}` is imported but not declared in package.json ({})",
                    fields
                        .iter()
                        .map(|f| format!("`{f}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                file: importer.path.to_path_buf(),
                location: import.location,
                modules: importer.module.into_iter().collect(),
                edge: None,
                fix: Some(format!(
                    "Add `{package}` to package.json, or add it to the rule's `ignore` if it is an alias or a virtual module"
                )),
            });
        }
    }
    out
}

fn is_type_only(import: &Import) -> bool {
    import.type_only
        || matches!(&import.bindings, Bindings::Names(names)
            if !names.is_empty() && names.iter().all(|n| n.type_only))
}

#[derive(Debug, PartialEq, Eq)]
enum Package<'a> {
    /// A Node builtin; a bare name (`buffer`) may also be a same-named npm polyfill.
    Builtin(Option<&'a str>),
    Npm(&'a str),
}

/// The package a bare specifier names; `None` for a relative or absolute path, an import that
/// resolved to a module (an alias), `#imports`, a URL or `virtual:` scheme, or a glob.
fn package_of(import: &Import) -> Option<Package<'_>> {
    if import.target.is_some() {
        return None;
    }
    let specifier = import
        .specifier
        .split(['?', '#'])
        .next()
        .unwrap_or_default();
    if let Some(builtin) = specifier.strip_prefix("node:") {
        return (!builtin.is_empty()).then_some(Package::Builtin(None));
    }
    let first = specifier.split('/').next().unwrap_or_default();
    let name = if first.starts_with('@') {
        let mut parts = specifier.splitn(3, '/');
        let (scope, package) = (parts.next()?, parts.next()?);
        if scope.len() < 2 || package.is_empty() {
            return None;
        }
        &specifier[..scope.len() + 1 + package.len()]
    } else {
        first
    };
    let valid = name
        .trim_start_matches('@')
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-._/".contains(c));
    if name.is_empty()
        || name.starts_with(['.', '_'])
        || !valid
        || import.specifier.contains(['*', '{'])
    {
        return None;
    }
    if NODE_BUILTINS.contains(&name) {
        return Some(Package::Builtin(Some(name)));
    }
    Some(Package::Npm(name))
}

const NODE_BUILTINS: &[&str] = &[
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "sys",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
    "test",
    "sqlite",
    "sea",
    "fs/promises",
    "path/posix",
    "path/win32",
    "stream/promises",
    "stream/web",
    "timers/promises",
    "util/types",
];

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use stratum_core::edge::EdgeKind;

    fn import(specifier: &str) -> Import {
        Import {
            specifier: specifier.to_owned(),
            location: SourceLocation { line: 1, column: 1 },
            kind: EdgeKind::Static,
            type_only: false,
            bindings: Bindings::None,
            target: None,
        }
    }

    #[test]
    fn a_bare_builtin_name_uses_a_declared_polyfill_of_that_name() {
        let mut graph = CompoundGraph::empty();
        graph.side_files.push(stratum_graph::SideFile {
            path: "src/main.ts".into(),
            relative: "src/main.ts".into(),
            role: SideRole::EntryPoint,
            imports: vec![import("buffer"), import("node:events")],
        });
        let declared = |package: &str| stratum_graph::Declared {
            field: "dependencies".into(),
            package: package.into(),
            location: SourceLocation { line: 1, column: 1 },
        };
        graph.manifest = Some(Manifest {
            declared: vec![declared("buffer"), declared("events")],
            ..Manifest::default()
        });
        let found: Vec<String> = UnusedDependencies
            .check(
                &graph,
                ProjectScope,
                &UnusedDependenciesOptions::default(),
                Severity::Error,
            )
            .into_iter()
            .map(|v| v.message)
            .collect();
        assert_eq!(
            found,
            ["Package `events` is declared in `dependencies` but nothing imports it"]
        );
    }

    #[test]
    fn a_bare_specifier_names_its_package_and_nothing_else_does() {
        let of = |s: &str| match package_of(&import(s)) {
            Some(Package::Npm(name)) => Some(name.to_owned()),
            Some(Package::Builtin(_)) => Some("<builtin>".to_owned()),
            None => None,
        };
        assert_eq!(of("solid-js/web").as_deref(), Some("solid-js"));
        assert_eq!(
            of("@tauri-apps/api/webview").as_deref(),
            Some("@tauri-apps/api")
        );
        assert_eq!(of("lodash?raw").as_deref(), Some("lodash"));
        assert_eq!(of("node:fs").as_deref(), Some("<builtin>"));
        assert_eq!(of("path").as_deref(), Some("<builtin>"));
        for none in [
            "./a",
            "../b",
            "/abs",
            "#internal",
            "@/alias",
            "~/x",
            "virtual:pwa",
            "@scope",
            "./*.ts",
            "C:\\x",
        ] {
            assert_eq!(of(none), None, "{none}");
        }
    }
}
