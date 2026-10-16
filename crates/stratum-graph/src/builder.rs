use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use camino::{Utf8Path, Utf8PathBuf};
use rustc_hash::FxHashMap;
use walkdir::WalkDir;

use stratum_core::edge::EdgeKind;
use stratum_core::ids::ModuleId;
use stratum_core::purity::Purity;
use stratum_core::structure::Structure;
use stratum_core::types::{Call, Exports, Layer, Module};
use stratum_core::violation::SourceLocation;
use stratum_parser_ts::{
    LanguageExtractor, Overlaid, Overlay, OxcTsExtractor, PathResolver, offset_to_line_col,
};

use crate::graph::CompoundGraph;
use crate::placement::place;

mod incremental;

/// On Windows, `canonicalize` returns extended-length paths (`\\?\D:\...`).
/// `oxc_resolver` returns the non-prefixed form. Strip the prefix so both
/// sides use a single canonical representation when used as `HashMap` keys.
fn strip_windows_extended_prefix(p: &Utf8Path) -> Utf8PathBuf {
    let s = p.as_str();
    Utf8PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(s))
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Could not read {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Project root {0} does not exist or is not a directory")]
    BadRoot(Utf8PathBuf),
    #[error("Parser error in {path}: {source}")]
    Parser {
        path: Utf8PathBuf,
        #[source]
        source: stratum_parser_ts::ExtractError,
    },
}

/// Configuration for graph construction.
///
/// The zero-config flow (Phase 5+) will synthesize `layers` from `src/*` automatically;
/// Phase 2 requires them upfront.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub project_root: Utf8PathBuf,
    pub layers: Vec<Layer>,
    pub structure: Structure,
    pub tracked_calls: HashSet<String>,
    pub default_purity: Purity,
}

/// Build a [`CompoundGraph`] for a given project.
pub struct GraphBuilder {
    config: BuildConfig,
    extractors: Vec<Box<dyn LanguageExtractor>>,
    resolver: PathResolver,
    ignore: IgnorePredicate,
    overlay: Arc<Overlay>,
}

type IgnorePredicate = Box<dyn Fn(&Utf8Path) -> bool + Send + Sync>;

impl std::fmt::Debug for GraphBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphBuilder")
            .field("config", &self.config)
            .field("extractors", &self.extractors.len())
            .finish_non_exhaustive()
    }
}

impl GraphBuilder {
    #[must_use]
    pub fn new(mut config: BuildConfig) -> Self {
        config.project_root = strip_windows_extended_prefix(&config.project_root);
        let resolver = PathResolver::new(&config.project_root);
        let ts = OxcTsExtractor::new(config.tracked_calls.clone());
        Self {
            config,
            extractors: vec![Box::new(ts)],
            resolver,
            ignore: Box::new(|_| false),
            overlay: Arc::default(),
        }
    }

    #[must_use]
    pub fn with_overlay(mut self, overlay: Overlay) -> Self {
        self.overlay = Arc::new(overlay);
        self.resolver =
            PathResolver::with_overlay(&self.config.project_root, Arc::clone(&self.overlay));
        self
    }

    /// `ignored` gets paths relative to the project root; a matching directory is not descended.
    #[must_use]
    pub fn with_ignore(
        mut self,
        ignored: impl Fn(&Utf8Path) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.ignore = Box::new(ignored);
        self
    }

    /// Register an additional language extractor. The first registered
    /// extractor whose `handles(path)` returns `true` wins.
    #[must_use]
    pub fn with_extractor(mut self, extractor: Box<dyn LanguageExtractor>) -> Self {
        self.extractors.push(extractor);
        self
    }

    fn pick_extractor(&self, path: &Utf8Path) -> Option<&dyn LanguageExtractor> {
        self.extractors
            .iter()
            .find(|e| e.handles(path))
            .map(std::convert::AsRef::as_ref)
    }

    /// Walk the project, extract imports per file, and assemble the graph.
    ///
    /// # Errors
    /// Returns [`BuildError::BadRoot`] if `project_root` is not a directory,
    /// [`BuildError::Io`] if a discovered source file cannot be read, or
    /// [`BuildError::Parser`] if the extractor fails on a file with a recognized extension.
    pub fn build(&self) -> Result<CompoundGraph, BuildError> {
        let root = &self.config.project_root;
        if !root.is_dir() {
            return Err(BuildError::BadRoot(root.clone()));
        }
        let mut graph = CompoundGraph::empty();
        for l in &self.config.layers {
            graph.layers.insert(l.id, l.clone());
        }

        let (files, sizes): (Vec<Utf8PathBuf>, Vec<u64>) = self
            .walk_sources(root)
            .into_iter()
            .filter(|(path, _)| self.pick_extractor(path).is_some())
            .unzip();
        let placement = place(root, &self.config.layers, &self.config.structure, &files);
        graph.containers = placement.containers;

        let paths: Vec<&Utf8Path> = placement
            .modules
            .iter()
            .map(|p| files[p.file].as_path())
            .collect();
        let sizes: Vec<u64> = placement.modules.iter().map(|p| sizes[p.file]).collect();
        let path_to_id: FxHashMap<Utf8PathBuf, ModuleId> = paths
            .iter()
            .enumerate()
            .map(|(i, p)| (p.to_path_buf(), module_id(i)))
            .collect();
        let mut facts: Vec<FileFacts> = self
            .scan_all(&paths, &sizes, &path_to_id)
            .into_iter()
            .collect::<Result<_, _>>()?;

        for ((i, placed), path) in placement.modules.into_iter().enumerate().zip(&paths) {
            let id = module_id(i);
            let module = Module {
                id,
                path: PathBuf::from(path.as_str()),
                container: placed.container,
                layer: placed.layer,
                purity: facts[i].purity.unwrap_or(self.config.default_purity),
                visibility: placed.visibility,
                exports: facts[i].exports,
                calls: std::mem::take(&mut facts[i].calls),
            };
            let node = graph.deps.add_node(id);
            graph.node_index.insert(id, node);
            graph.modules.insert(id, module);
        }

        for (i, f) in facts.into_iter().enumerate() {
            let from_node = graph.node_index[&module_id(i)];
            for (to_id, kind) in f.edges {
                let to_node = graph.node_index[&to_id];
                graph.deps.add_edge(from_node, to_node, kind);
            }
        }

        Ok(graph)
    }

    fn walk_sources(&self, root: &Utf8Path) -> Vec<(Utf8PathBuf, u64)> {
        let root_std = root.as_std_path();
        let layer_paths: Vec<&Path> = self
            .config
            .layers
            .iter()
            .map(|l| l.path.as_path())
            .collect();
        let reachable = |dir: &Path| {
            dir.strip_prefix(root_std).map_or(true, |rel| {
                layer_paths
                    .iter()
                    .any(|lp| rel.starts_with(lp) || lp.starts_with(rel))
            })
        };
        let ignored = |path: &Path| {
            path.strip_prefix(root_std)
                .ok()
                .and_then(Utf8Path::from_path)
                .is_some_and(|rel| (self.ignore)(rel))
        };
        let mut paths: Vec<(Utf8PathBuf, u64)> = WalkDir::new(root_std)
            .into_iter()
            .filter_entry(|e| {
                !ignored(e.path()) && (!e.file_type().is_dir() || reachable(e.path()))
            })
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file() && self.overlay.get(e.path()) == Overlaid::Disk)
            .filter_map(|e| {
                let size = e.metadata().map_or(0, |m| m.len());
                let path = Utf8PathBuf::from_path_buf(e.into_path()).ok()?;
                Some((strip_windows_extended_prefix(&path), size))
            })
            .collect();
        let walked = |file: &Path| {
            file.starts_with(root_std)
                && !ignored(file)
                && file
                    .ancestors()
                    .skip(1)
                    .take_while(|dir| dir.starts_with(root_std) && *dir != root_std)
                    .all(|dir| !ignored(dir) && reachable(dir))
        };
        paths.extend(self.overlay.present().filter_map(|(path, text)| {
            let path = Utf8Path::from_path(path).filter(|p| walked(p.as_std_path()))?;
            Some((
                path.to_path_buf(),
                u64::try_from(text.len()).unwrap_or(u64::MAX),
            ))
        }));
        paths.sort_unstable();
        paths
    }

    fn scan_all(
        &self,
        paths: &[&Utf8Path],
        sizes: &[u64],
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
    ) -> Vec<Scan> {
        let mut largest_first: Vec<usize> = (0..paths.len()).collect();
        largest_first.sort_by_key(|&i| std::cmp::Reverse(sizes[i]));
        let workers = std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get)
            .min(MAX_WORKERS)
            .min(paths.len().max(1));
        let next = AtomicUsize::new(0);
        let large_file = Mutex::new(());
        let mut indexed: Vec<(usize, Scan)> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    s.spawn(|| {
                        let mut out = Vec::new();
                        loop {
                            let Some(&i) = largest_first.get(next.fetch_add(1, Ordering::Relaxed))
                            else {
                                break;
                            };
                            out.push((i, self.scan(paths[i], path_to_id, &large_file)));
                        }
                        out
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_default())
                .collect()
        });
        indexed.sort_unstable_by_key(|(i, _)| *i);
        indexed.into_iter().map(|(_, scan)| scan).collect()
    }

    fn scan(
        &self,
        path: &Utf8Path,
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
        large_file: &Mutex<()>,
    ) -> Scan {
        let source = match self.overlay.get(path.as_std_path()) {
            Overlaid::Text(text) => text.to_owned(),
            _ => fs::read_to_string(path.as_std_path()).map_err(|e| BuildError::Io {
                path: path.to_path_buf(),
                source: e,
            })?,
        };
        let _serial = (source.len() > LARGE_FILE_BYTES).then(|| large_file.lock());
        self.facts_of(path, &source, path_to_id)
    }

    fn facts_of(
        &self,
        path: &Utf8Path,
        source: &str,
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
    ) -> Result<FileFacts, BuildError> {
        let extractor = self
            .pick_extractor(path)
            .ok_or_else(|| BuildError::Parser {
                path: path.to_path_buf(),
                source: stratum_parser_ts::ExtractError::UnsupportedExtension(path.to_path_buf()),
            })?;
        let data = extractor
            .extract(path, source)
            .map_err(|e| BuildError::Parser {
                path: path.to_path_buf(),
                source: e,
            })?;
        let edges = data
            .imports
            .iter()
            .filter_map(|imp| {
                let target = self.resolver.resolve(path, &imp.specifier).ok()?;
                let id = path_to_id.get(&strip_windows_extended_prefix(&target))?;
                Some((*id, imp.kind))
            })
            .collect();
        let calls = data
            .calls
            .into_iter()
            .map(|call| {
                let (line, column) = offset_to_line_col(source, call.span.start);
                Call {
                    name: call.name,
                    location: SourceLocation { line, column },
                }
            })
            .collect();
        Ok(FileFacts {
            purity: data.purity,
            exports: data.exports,
            edges,
            calls,
        })
    }
}

fn module_id(index: usize) -> ModuleId {
    ModuleId::new(u32::try_from(index).unwrap_or(u32::MAX))
}

const LARGE_FILE_BYTES: usize = 1 << 20;
const MAX_WORKERS: usize = 8;

struct FileFacts {
    purity: Option<Purity>,
    exports: Exports,
    edges: Vec<(ModuleId, EdgeKind)>,
    calls: Vec<Call>,
}

type Scan = Result<FileFacts, BuildError>;
