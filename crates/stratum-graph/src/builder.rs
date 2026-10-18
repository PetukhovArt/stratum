use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};

use camino::{Utf8Path, Utf8PathBuf};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use stratum_core::edge::EdgeKind;
use stratum_core::ids::ModuleId;
use stratum_core::purity::Purity;
use stratum_core::structure::Structure;
use stratum_core::types::{
    Bindings, Call, ExportDecl, ExportOrigin, Exports, Import, Layer, Module, Suppression,
};
use stratum_core::violation::SourceLocation;
use stratum_parser_ts::{
    ExtractedData, LanguageExtractor, LineIndex, Overlaid, Overlay, OxcTsExtractor, PathResolver,
    TrackedCalls,
};

use crate::graph::CompoundGraph;
use crate::placement::place;

use cache::{Cache, Prior, Stat, Written};
pub use cache::{CacheState, CacheStats};

mod cache;
mod incremental;

/// On Windows, `canonicalize` returns extended-length paths (`\\?\D:\...`).
/// `oxc_resolver` returns the non-prefixed form. Strip the prefix so both
/// sides use a single canonical representation when used as `HashMap` keys.
pub(crate) fn strip_windows_extended_prefix(p: &Utf8Path) -> Utf8PathBuf {
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
    pub tracked_calls: TrackedCalls,
    pub default_purity: Purity,
}

/// Build a [`CompoundGraph`] for a given project.
pub struct GraphBuilder {
    config: BuildConfig,
    extractors: Vec<Box<dyn LanguageExtractor>>,
    resolver: PathResolver,
    ignore: IgnorePredicate,
    overlay: Arc<Overlay>,
    overlaid: bool,
    cache: Option<Cache>,
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
            overlaid: false,
            cache: None,
        }
    }

    /// A build with an overlay never reads or writes the Graph Cache.
    #[must_use]
    pub fn with_overlay(mut self, overlay: Overlay) -> Self {
        self.overlay = Arc::new(overlay);
        self.overlaid = true;
        self.resolver =
            PathResolver::with_overlay(&self.config.project_root, Arc::clone(&self.overlay));
        self
    }

    /// Keep the Graph Cache in `dir` (`None`: off). `identity` tells configs of one root apart,
    /// e.g. the config path; each pair gets its own cache file.
    #[must_use]
    pub fn with_cache_dir(mut self, dir: Option<Utf8PathBuf>, identity: &str) -> Self {
        self.cache = dir.map(|dir| {
            Cache::new(
                &dir,
                &self.config.project_root,
                identity,
                &self.config.tracked_calls,
            )
        });
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
    pub fn build(&self) -> Result<(CompoundGraph, CacheStats), BuildError> {
        let root = &self.config.project_root;
        if !root.is_dir() {
            return Err(BuildError::BadRoot(root.clone()));
        }
        let mut graph = CompoundGraph::empty();
        for l in &self.config.layers {
            graph.layers.insert(l.id, l.clone());
        }

        let walked = self.walk_sources(root);
        let (files, stamps): (Vec<Utf8PathBuf>, Vec<Stat>) = walked
            .iter()
            .filter(|(path, _)| self.pick_extractor(path).is_some())
            .cloned()
            .unzip();
        let placement = place(root, &self.config.layers, &self.config.structure, &files);
        graph.containers = placement.containers;
        graph.dir_roles = placement.dir_roles;

        let paths: Vec<&Utf8Path> = placement
            .modules
            .iter()
            .map(|p| files[p.file].as_path())
            .collect();
        let stamps: Vec<Stat> = placement.modules.iter().map(|p| stamps[p.file]).collect();
        let path_to_id: FxHashMap<Utf8PathBuf, ModuleId> = paths
            .iter()
            .enumerate()
            .map(|(i, p)| (p.to_path_buf(), module_id(i)))
            .collect();
        let CachedScans {
            facts,
            stats: cache_stats,
            pending,
        } = self.scan_cached(&walked, &paths, &stamps, &path_to_id)?;

        for ((i, placed), (path, facts)) in placement
            .modules
            .into_iter()
            .enumerate()
            .zip(paths.iter().zip(facts))
        {
            let id = module_id(i);
            let module = Module {
                id,
                path: PathBuf::from(path.as_str()),
                container: placed.container,
                layer: placed.layer,
                purity: facts.purity.unwrap_or(self.config.default_purity),
                visibility: placed.visibility,
                exports: facts.exports,
                calls: facts.calls,
                imports: facts.imports,
                suppressions: facts.suppressions,
            };
            let node = graph.deps.add_node(id);
            graph.node_index.insert(id, node);
            graph.modules.insert(id, module);
        }

        for i in 0..paths.len() {
            let id = module_id(i);
            let from_node = graph.node_index[&id];
            for (to_id, kind) in edges(&graph.modules[&id].imports) {
                let to_node = graph.node_index[&to_id];
                graph.deps.add_edge(from_node, to_node, kind);
            }
        }

        if let Some((cache, bytes)) = pending {
            cache.write(&bytes);
        }
        Ok((graph, cache_stats))
    }

    fn scan_cached(
        &self,
        walked: &[(Utf8PathBuf, Stat)],
        paths: &[&Utf8Path],
        stamps: &[Stat],
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
    ) -> Result<CachedScans<'_>, BuildError> {
        let cache = self.cache.as_ref().filter(|_| !self.overlaid);
        let env = cache.map(|_| {
            let walked = walked.iter().map(|(p, _)| p.as_path());
            cache::env_fingerprint(&self.config.project_root, paths, walked)
        });
        let loaded = cache.and_then(Cache::load);
        let state = match (cache, &loaded) {
            (None, _) => CacheState::Off,
            (Some(_), None) => CacheState::Cold,
            (Some(_), Some(_)) => CacheState::Warm,
        };
        let same_env = loaded.as_ref().is_some_and(|l| Some(l.env) == env);
        let priors: Vec<Prior<'_>> = paths
            .iter()
            .zip(stamps)
            .map(|(path, &stamp)| {
                loaded
                    .as_ref()
                    .map_or(Prior::Miss, |l| l.prior(path.as_str(), stamp))
            })
            .collect();
        let job = Job {
            priors: &priors,
            same_env,
            blobs: cache.is_some(),
            path_to_id,
        };
        let scanned: Vec<Scanned<'_>> = self
            .scan_all(paths, stamps, &job)
            .into_iter()
            .collect::<Result<_, _>>()?;
        let summary = CacheStats {
            state,
            reparsed: scanned.iter().filter(|s| s.reparsed).count(),
        };
        let rewrite = state != CacheState::Warm || !same_env || scanned.iter().any(|s| s.dirty);
        let pending = cache.zip(env).filter(|_| rewrite).and_then(|(cache, env)| {
            let modules: Option<Vec<Written<'_>>> = scanned
                .iter()
                .zip(paths)
                .zip(stamps)
                .map(|((s, path), &stat)| {
                    Some(Written {
                        path: path.as_str(),
                        stat,
                        hash: s.hash,
                        blob: s.blob.as_deref()?,
                    })
                })
                .collect();
            Some((cache, cache.encode(env, &modules?)?))
        });
        Ok(CachedScans {
            facts: scanned.into_iter().map(|s| s.facts).collect(),
            stats: summary,
            pending,
        })
    }

    fn walk_sources(&self, root: &Utf8Path) -> Vec<(Utf8PathBuf, Stat)> {
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
        let mut paths: Vec<(Utf8PathBuf, Stat)> = if ignored(root_std) {
            Vec::new()
        } else {
            let wanted = |file: &Path| {
                Utf8Path::from_path(file).is_some_and(|file| {
                    self.pick_extractor(file).is_some() || cache::shapes_resolution(root, file)
                })
            };
            walk(root_std, &|dir| !ignored(dir) && reachable(dir), &|file| {
                wanted(file) && !ignored(file) && self.overlay.get(file) == Overlaid::Disk
            })
            .into_iter()
            .filter_map(|(path, stat)| {
                let path = Utf8PathBuf::from_path_buf(path).ok()?;
                Some((strip_windows_extended_prefix(&path), stat))
            })
            .collect()
        };
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
                Stat {
                    size: u64::try_from(text.len()).unwrap_or(u64::MAX),
                    mtime_ns: 0,
                },
            ))
        }));
        paths.sort_by_cached_key(|(path, _)| component_order(path));
        paths
    }

    fn scan_all<'c>(
        &self,
        paths: &[&Utf8Path],
        stats: &[Stat],
        job: &Job<'_, 'c>,
    ) -> Vec<Result<Scanned<'c>, BuildError>> {
        let mut largest_first: Vec<usize> = (0..paths.len()).collect();
        largest_first.sort_by_key(|&i| std::cmp::Reverse(stats[i].size));
        let workers = std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get)
            .min(MAX_WORKERS)
            .min(paths.len().max(1));
        let next = AtomicUsize::new(0);
        let large_file = Mutex::new(());
        let mut indexed: Vec<(usize, Result<Scanned<'c>, BuildError>)> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    s.spawn(|| {
                        let mut out = Vec::new();
                        loop {
                            let Some(&i) = largest_first.get(next.fetch_add(1, Ordering::Relaxed))
                            else {
                                break;
                            };
                            out.push((i, self.scan(paths[i], job.priors[i], job, &large_file)));
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

    fn scan<'c>(
        &self,
        path: &Utf8Path,
        prior: Prior<'c>,
        job: &Job<'_, 'c>,
        large_file: &Mutex<()>,
    ) -> Result<Scanned<'c>, BuildError> {
        let (cached, source) = match prior {
            Prior::Fresh(c) => (cache::decode_facts(c.blob).map(|f| (f, c)), None),
            Prior::Verify(c) => {
                let source = self.source(path)?;
                let same = cache::hash(&source) == c.hash;
                let cached = same.then(|| cache::decode_facts(c.blob)).flatten();
                (cached.map(|f| (f, c)), Some(source))
            }
            Prior::Miss => (None, None),
        };
        let encode = |parsed: &Parsed, targets: &[Vec<ModuleId>]| {
            job.blobs
                .then(|| cache::encode_facts(parsed, targets))
                .flatten()
                .map(Cow::Owned)
        };
        if let Some(((parsed, targets), c)) = cached {
            let (targets, blob) = if job.same_env {
                (targets, job.blobs.then_some(Cow::Borrowed(c.blob)))
            } else {
                let targets = self.resolve(path, &parsed, job.path_to_id);
                let blob = encode(&parsed, &targets);
                (targets, blob)
            };
            return Ok(Scanned {
                facts: assemble(parsed, targets),
                blob,
                hash: c.hash,
                reparsed: false,
                dirty: !matches!(prior, Prior::Fresh(_)),
            });
        }
        let source = match source {
            Some(source) => source,
            None => self.source(path)?,
        };
        let parsed = self.parse_large(path, &source, large_file)?;
        let targets = self.resolve(path, &parsed, job.path_to_id);
        let blob = encode(&parsed, &targets);
        Ok(Scanned {
            facts: assemble(parsed, targets),
            blob,
            hash: cache::hash(&source),
            reparsed: true,
            dirty: true,
        })
    }

    fn source(&self, path: &Utf8Path) -> Result<String, BuildError> {
        match self.overlay.get(path.as_std_path()) {
            Overlaid::Text(text) => Ok(text.to_owned()),
            _ => fs::read_to_string(path.as_std_path()).map_err(|e| BuildError::Io {
                path: path.to_path_buf(),
                source: e,
            }),
        }
    }

    fn parse_large(
        &self,
        path: &Utf8Path,
        source: &str,
        large_file: &Mutex<()>,
    ) -> Result<Parsed, BuildError> {
        let _serial = (source.len() > LARGE_FILE_BYTES).then(|| large_file.lock());
        self.parse(path, source)
    }

    fn facts_of(
        &self,
        path: &Utf8Path,
        source: &str,
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
    ) -> Result<FileFacts, BuildError> {
        let parsed = self.parse(path, source)?;
        let targets = self.resolve(path, &parsed, path_to_id);
        Ok(assemble(parsed, targets))
    }

    fn parse(&self, path: &Utf8Path, source: &str) -> Result<Parsed, BuildError> {
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
        Ok(located(&LineIndex::new(source), data))
    }

    fn resolve(
        &self,
        path: &Utf8Path,
        parsed: &Parsed,
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
    ) -> Vec<Vec<ModuleId>> {
        parsed
            .imports
            .iter()
            .map(|raw| {
                if raw.glob {
                    self.glob_targets(path, &raw.specifier, path_to_id)
                } else {
                    self.resolver
                        .resolve(path, &raw.specifier)
                        .ok()
                        .and_then(|t| path_to_id.get(&strip_windows_extended_prefix(&t)).copied())
                        .into_iter()
                        .collect()
                }
            })
            .collect()
    }

    /// Modules an `import.meta.glob` pattern matches, the importer excluded. Relative patterns
    /// and `/`-rooted ones (from ROOT) are resolved; alias patterns match nothing.
    fn glob_targets(
        &self,
        from: &Utf8Path,
        pattern: &str,
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
    ) -> Vec<ModuleId> {
        let root = &self.config.project_root;
        let base: Vec<&str> = if pattern.starts_with('/') {
            Vec::new()
        } else if pattern.starts_with("./") || pattern.starts_with("../") {
            let Some(dir) = from.parent().and_then(|d| d.strip_prefix(root).ok()) else {
                return Vec::new();
            };
            dir.components().map(|c| c.as_str()).collect()
        } else {
            return Vec::new();
        };
        let Some(relative) = normalized(base, pattern) else {
            return Vec::new();
        };
        let Ok(glob) = globset::GlobBuilder::new(&relative)
            .literal_separator(true)
            .build()
        else {
            return Vec::new();
        };
        let matcher = glob.compile_matcher();
        let mut ids: Vec<ModuleId> = path_to_id
            .iter()
            .filter(|(path, _)| path.as_path() != from)
            .filter(|(path, _)| {
                path.strip_prefix(root)
                    .is_ok_and(|rel| matcher.is_match(rel.as_str().replace('\\', "/")))
            })
            .map(|(_, id)| *id)
            .collect();
        ids.sort_unstable();
        ids
    }
}

fn normalized<'p>(mut segments: Vec<&'p str>, pattern: &'p str) -> Option<String> {
    for segment in pattern.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            s => segments.push(s),
        }
    }
    Some(segments.join("/"))
}

fn located(lines: &LineIndex<'_>, data: ExtractedData) -> Parsed {
    let at = |offset: u32| {
        let (line, column) = lines.line_col(offset);
        SourceLocation { line, column }
    };
    let imports = data
        .imports
        .into_iter()
        .map(|raw| ParsedImport {
            location: at(raw.statement.start),
            specifier: raw.specifier,
            kind: raw.kind,
            type_only: raw.type_only,
            bindings: raw.bindings,
            glob: raw.glob,
        })
        .collect();
    let exports = Exports {
        decls: data
            .exports
            .decls
            .into_iter()
            .map(|d| ExportDecl {
                name: d.name,
                type_only: d.type_only,
                location: at(d.span.start),
                origin: d.origin,
            })
            .collect(),
        stars: data.exports.stars,
        unimportable: data.exports.unimportable,
    };
    let calls = data
        .calls
        .into_iter()
        .map(|call| Call {
            name: call.name,
            location: at(call.span.start),
        })
        .collect();
    let suppressions = data
        .suppressions
        .into_iter()
        .map(|s| Suppression {
            location: at(s.span.start),
            applies_to: at(s.span.end).line.saturating_add(1),
            slugs: s.slugs,
            reason: s.reason,
        })
        .collect();
    Parsed {
        purity: data.purity,
        imports,
        exports,
        calls,
        suppressions,
    }
}

fn assemble(parsed: Parsed, targets_per_import: Vec<Vec<ModuleId>>) -> FileFacts {
    let mut imports = Vec::with_capacity(parsed.imports.len());
    let mut first_of_raw = Vec::with_capacity(parsed.imports.len());
    for (raw, targets) in parsed.imports.into_iter().zip(targets_per_import) {
        first_of_raw.push(imports.len());
        let import = |target, specifier, bindings| Import {
            specifier,
            location: raw.location,
            kind: raw.kind,
            type_only: raw.type_only,
            bindings,
            target,
        };
        let (last, rest) = match targets.split_last() {
            Some((&last, rest)) => (Some(last), rest),
            None => (None, &[][..]),
        };
        for &t in rest {
            imports.push(import(Some(t), raw.specifier.clone(), raw.bindings.clone()));
        }
        imports.push(import(last, raw.specifier, raw.bindings));
    }
    let mut exports = parsed.exports;
    for decl in &mut exports.decls {
        if let ExportOrigin::Reexport { import, .. } = &mut decl.origin {
            *import = first_of_raw[*import];
        }
    }
    for star in &mut exports.stars {
        *star = first_of_raw[*star];
    }
    FileFacts {
        purity: parsed.purity,
        exports,
        imports,
        calls: parsed.calls,
        suppressions: parsed.suppressions,
    }
}

fn component_order(path: &Utf8Path) -> Vec<u8> {
    path.as_str()
        .bytes()
        .map(|b| {
            if std::path::is_separator(char::from(b)) {
                0
            } else {
                b
            }
        })
        .collect()
}

fn walk(
    root: &Path,
    enter: &(dyn Fn(&Path) -> bool + Sync),
    take: &(dyn Fn(&Path) -> bool + Sync),
) -> Vec<(PathBuf, Stat)> {
    struct Queue {
        dirs: Vec<PathBuf>,
        busy: usize,
    }
    let queue = Mutex::new(Queue {
        dirs: vec![root.to_path_buf()],
        busy: 0,
    });
    let wake = Condvar::new();
    let next_dir = || {
        let mut q = queue.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if let Some(dir) = q.dirs.pop() {
                q.busy += 1;
                return Some(dir);
            }
            if q.busy == 0 {
                wake.notify_all();
                return None;
            }
            q = wake.wait(q).unwrap_or_else(PoisonError::into_inner);
        }
    };
    let workers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(MAX_WORKERS);
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                s.spawn(|| {
                    let mut files = Vec::new();
                    while let Some(dir) = next_dir() {
                        let mut subdirs = Vec::new();
                        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
                            let Ok(kind) = entry.file_type() else {
                                continue;
                            };
                            let path = entry.path();
                            if kind.is_dir() {
                                if enter(&path) {
                                    subdirs.push(path);
                                }
                            } else if kind.is_file() && take(&path) {
                                let stat = entry.metadata().map_or(
                                    Stat {
                                        size: 0,
                                        mtime_ns: 0,
                                    },
                                    |m| Stat::of(&m),
                                );
                                files.push((path, stat));
                            }
                        }
                        let mut q = queue.lock().unwrap_or_else(PoisonError::into_inner);
                        q.dirs.extend(subdirs);
                        q.busy -= 1;
                        wake.notify_all();
                    }
                    files
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

pub(crate) fn edges(imports: &[Import]) -> impl Iterator<Item = (ModuleId, EdgeKind)> + '_ {
    imports.iter().filter_map(|i| Some((i.target?, i.kind)))
}

fn module_id(index: usize) -> ModuleId {
    ModuleId::new(u32::try_from(index).unwrap_or(u32::MAX))
}

const LARGE_FILE_BYTES: usize = 1 << 20;
const MAX_WORKERS: usize = 8;

struct FileFacts {
    purity: Option<Purity>,
    exports: Exports,
    imports: Vec<Import>,
    calls: Vec<Call>,
    suppressions: Vec<Suppression>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Parsed {
    purity: Option<Purity>,
    imports: Vec<ParsedImport>,
    exports: Exports,
    calls: Vec<Call>,
    suppressions: Vec<Suppression>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedImport {
    specifier: String,
    location: SourceLocation,
    kind: EdgeKind,
    type_only: bool,
    bindings: Bindings,
    glob: bool,
}

struct Scanned<'c> {
    facts: FileFacts,
    blob: Option<Cow<'c, [u8]>>,
    hash: u64,
    reparsed: bool,
    dirty: bool,
}

struct Job<'a, 'c> {
    priors: &'a [Prior<'c>],
    same_env: bool,
    blobs: bool,
    path_to_id: &'a FxHashMap<Utf8PathBuf, ModuleId>,
}

struct CachedScans<'c> {
    facts: Vec<FileFacts>,
    stats: CacheStats,
    pending: Option<(&'c Cache, Vec<u8>)>,
}
