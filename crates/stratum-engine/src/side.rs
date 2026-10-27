//! Side files: source files outside the graph that the engine reads for their imports only.

use std::path::PathBuf;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, IgnoreSet};
use stratum_core::severity::Severity;
use stratum_graph::{CompoundGraph, GraphBuilder, SideFile, SideRole};
use stratum_rules::{UnusedDependenciesOptions, UnusedExportsOptions};

/// Where to look for side files: the layer folders, and what is ignored inside and out.
pub(crate) struct Walk<'a> {
    root: &'a Utf8Path,
    layers: Vec<String>,
    ignore: &'a IgnoreSet,
}

impl<'a> Walk<'a> {
    pub(crate) fn new(root: &'a Utf8Path, config: &Config, ignore: &'a IgnoreSet) -> Self {
        let layers = config
            .layers
            .iter()
            .map(|l| {
                let path = l.path.as_str().replace('\\', "/");
                path.trim_start_matches("./")
                    .trim_end_matches('/')
                    .to_owned()
            })
            .collect();
        Self {
            root,
            layers,
            ignore,
        }
    }

    fn in_layer(&self, rel: &str) -> bool {
        self.layers.iter().any(|layer| {
            layer.is_empty()
                || layer == "."
                || rel == layer
                || rel
                    .strip_prefix(layer.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
    }

    /// Source files below `base` (relative to the root) outside every layer and not ignored,
    /// relative to the root with `/`. `max_depth` counts levels below `base`: `Some(1)` takes
    /// only the files directly in it.
    pub(crate) fn outside_layers(&self, base: &str, max_depth: Option<usize>) -> Vec<String> {
        let mut out = Vec::new();
        if !base.is_empty() && (self.in_layer(base) || self.is_ignored(base)) {
            return out;
        }
        let mut keep = |rel: &str, is_dir: bool| {
            if is_dir {
                !self.in_layer(rel) && !self.is_ignored(rel)
            } else {
                !self.is_ignored(rel) && crate::is_source(Utf8Path::new(rel))
            }
        };
        self.descend(base, max_depth, &mut keep, &mut out);
        out
    }

    /// Source files inside the layers that `project.ignore` ignores and the built-in defaults
    /// (`node_modules`, `dist`, ...) do not, relative to the root with `/`.
    pub(crate) fn ignored_in_layers(&self) -> Vec<String> {
        let defaults = IgnoreSet::defaults();
        let mut out = Vec::new();
        let mut keep = |rel: &str, is_dir: bool| {
            let defaulted = defaults.is_ignored(Utf8Path::new(rel));
            if is_dir {
                !defaulted
            } else {
                !defaulted && self.is_ignored(rel) && crate::is_source(Utf8Path::new(rel))
            }
        };
        for layer in &self.layers {
            let base = if layer == "." { "" } else { layer.as_str() };
            if base.is_empty() || !defaults.is_ignored(Utf8Path::new(base)) {
                self.descend(base, None, &mut keep, &mut out);
            }
        }
        out.sort();
        out.dedup();
        out
    }

    fn is_ignored(&self, rel: &str) -> bool {
        self.ignore.is_ignored(Utf8Path::new(rel))
    }

    fn descend(
        &self,
        dir: &str,
        depth_left: Option<usize>,
        keep: &mut dyn FnMut(&str, bool) -> bool,
        out: &mut Vec<String>,
    ) {
        if depth_left == Some(0) {
            return;
        }
        let abs = if dir.is_empty() {
            self.root.to_path_buf()
        } else {
            self.root.join(dir)
        };
        let Ok(entries) = std::fs::read_dir(abs.as_std_path()) else {
            return;
        };
        let mut children: Vec<(String, bool)> = entries
            .flatten()
            .filter_map(|entry| {
                let kind = entry.file_type().ok()?;
                let name = entry.file_name().into_string().ok()?;
                let rel = if dir.is_empty() {
                    name
                } else {
                    format!("{dir}/{name}")
                };
                (kind.is_dir() || kind.is_file()).then_some((rel, kind.is_dir()))
            })
            .collect();
        children.sort();
        for (rel, is_dir) in children {
            if !keep(&rel, is_dir) {
                continue;
            }
            if is_dir {
                self.descend(&rel, depth_left.map(|d| d - 1), keep, out);
            } else {
                out.push(rel);
            }
        }
    }
}

const UNUSED_EXPORTS: &str = "stratum/unused-exports";
pub(crate) const UNUSED_DEPENDENCIES: &str = "stratum/unused-dependencies";

/// Whether a rule reads the ignored files inside the layers: `stratum/unused-exports` with
/// `count_ignored_importers`, or `stratum/unused-dependencies`.
pub(crate) fn wants_ignored(config: &Config) -> bool {
    let counted = enabled(config, UNUSED_EXPORTS)
        && options::<UnusedExportsOptions>(config, UNUSED_EXPORTS)
            .is_some_and(|o| o.count_ignored_importers);
    counted || enabled(config, UNUSED_DEPENDENCIES)
}

/// The `entries` globs of `stratum/unused-dependencies`; empty while it is off.
pub(crate) fn listed(config: &Config) -> Vec<String> {
    if !enabled(config, UNUSED_DEPENDENCIES) {
        return Vec::new();
    }
    options::<UnusedDependenciesOptions>(config, UNUSED_DEPENDENCIES)
        .unwrap_or_default()
        .entries
}

impl Walk<'_> {
    /// Files outside the layers that a glob of `patterns` matches; `*` stays within one folder.
    pub(crate) fn matching(&self, patterns: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        for pattern in patterns {
            let Ok(glob) = globset::GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
            else {
                continue;
            };
            let glob = glob.compile_matcher();
            let base = stratum_config::literal_dir(pattern);
            let depth = (!pattern.contains("**")).then(|| {
                let below = pattern.split('/').count();
                let at = if base.is_empty() {
                    0
                } else {
                    base.split('/').count()
                };
                below - at
            });
            out.extend(
                self.outside_layers(&base, depth)
                    .into_iter()
                    .filter(|rel| glob.is_match(rel)),
            );
        }
        out
    }
}

/// The rule is on in `rules` or in some `overrides` block. Both built-ins that read side files
/// are off by default, so one absent from `rules` is off.
pub(crate) fn enabled(config: &Config, slug: &str) -> bool {
    config
        .rules
        .get(slug)
        .is_some_and(|rule| rule.severity != Severity::Off)
        || config
            .overrides
            .iter()
            .filter_map(|o| o.rules.get(slug))
            .any(|rule| rule.severity != Severity::Off)
}

pub(crate) fn options<T: serde::de::DeserializeOwned>(config: &Config, slug: &str) -> Option<T> {
    let rule = config.rules.get(slug)?;
    if rule.options.is_null() {
        return None;
    }
    serde_json::from_value(rule.options.clone()).ok()
}

/// Parse `files` (relative to `root`) with `builder` for their imports; a file no extractor
/// handles or that does not parse is left out. Sorted by path, one entry per path.
pub(crate) fn scan(
    builder: &GraphBuilder,
    graph: &CompoundGraph,
    root: &Utf8Path,
    mut files: Vec<(String, SideRole)>,
) -> Vec<SideFile> {
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|later, first| later.0 == first.0);
    let paths: Vec<Utf8PathBuf> = files
        .iter()
        .map(|(rel, _)| {
            rel.split('/')
                .fold(root.to_path_buf(), |path, segment| path.join(segment))
        })
        .collect();
    let imports = builder.scan_side(graph, &paths);
    files
        .into_iter()
        .zip(paths)
        .zip(imports)
        .filter_map(|(((relative, role), path), imports)| {
            Some(SideFile {
                path: PathBuf::from(path.into_string()),
                relative,
                role,
                imports: imports?,
            })
        })
        .collect()
}
