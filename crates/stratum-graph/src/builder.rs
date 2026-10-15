use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use camino::{Utf8Path, Utf8PathBuf};
use rustc_hash::FxHashMap;
use walkdir::WalkDir;

use stratum_core::edge::EdgeKind;
use stratum_core::ids::{ContainerId, LayerId, ModuleId};
use stratum_core::stage::Stage;
use stratum_core::types::{Container, Layer, Module};
use stratum_core::visibility::VisibilityScope;
use stratum_parser_ts::{LanguageExtractor, OxcTsExtractor, PathResolver};

use crate::graph::CompoundGraph;
use crate::layer_assignment::assign_layer;

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
    pub default_stage: Stage,
    pub default_visibility: VisibilityScope,
}

/// Build a [`CompoundGraph`] for a given project.
pub struct GraphBuilder {
    config: BuildConfig,
    extractors: Vec<Box<dyn LanguageExtractor>>,
    resolver: PathResolver,
    ignore: IgnorePredicate,
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
        Self {
            config,
            extractors: vec![Box::new(OxcTsExtractor::new())],
            resolver,
            ignore: Box::new(|_| false),
        }
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
            let cid = ContainerId::new(l.id.raw());
            graph.containers.insert(
                cid,
                Container {
                    id: cid,
                    name: l.name.clone(),
                    layer: l.id,
                    parent: None,
                },
            );
        }

        let mut modules: Vec<(ModuleId, LayerId, Utf8PathBuf)> = Vec::new();
        let mut sizes: Vec<u64> = Vec::new();
        let mut path_to_id: FxHashMap<Utf8PathBuf, ModuleId> = FxHashMap::default();
        for (path, size) in self.walk_sources(root) {
            if self.pick_extractor(&path).is_none() {
                continue;
            }
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let Some(layer) = assign_layer(&self.config.layers, rel) else {
                continue;
            };
            let id = ModuleId::new(u32::try_from(modules.len()).unwrap_or(u32::MAX));
            path_to_id.insert(path.clone(), id);
            modules.push((id, layer, path));
            sizes.push(size);
        }

        let scanned = self.scan_all(&modules, &sizes, &path_to_id);

        for ((id, layer, path), scan) in modules.iter().zip(&scanned) {
            let module = Module {
                id: *id,
                path: PathBuf::from(path.as_str()),
                container: ContainerId::new(layer.raw()),
                layer: *layer,
                stage: scan.stage.unwrap_or(self.config.default_stage),
                visibility: self.config.default_visibility.clone(),
            };
            let node = graph.deps.add_node(*id);
            graph.node_index.insert(*id, node);
            graph.modules.insert(*id, module);
        }

        for ((from_id, _, _), scan) in modules.iter().zip(scanned) {
            let edges = scan.edges?;
            let from_node = graph.node_index[from_id];
            for (to_id, kind) in edges {
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
        let mut paths: Vec<(Utf8PathBuf, u64)> = WalkDir::new(root_std)
            .into_iter()
            .filter_entry(|e| {
                let ignored = e
                    .path()
                    .strip_prefix(root_std)
                    .ok()
                    .and_then(Utf8Path::from_path)
                    .is_some_and(|rel| (self.ignore)(rel));
                !ignored && (!e.file_type().is_dir() || reachable(e.path()))
            })
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| {
                let size = e.metadata().map_or(0, |m| m.len());
                let path = Utf8PathBuf::from_path_buf(e.into_path()).ok()?;
                Some((strip_windows_extended_prefix(&path), size))
            })
            .collect();
        paths.sort_unstable();
        paths
    }

    fn scan_all(
        &self,
        modules: &[(ModuleId, LayerId, Utf8PathBuf)],
        sizes: &[u64],
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
    ) -> Vec<Scan> {
        let mut largest_first: Vec<usize> = (0..modules.len()).collect();
        largest_first.sort_by_key(|&i| std::cmp::Reverse(sizes[i]));
        let workers = std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get)
            .min(MAX_WORKERS)
            .min(modules.len().max(1));
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
                            let (_, _, path) = &modules[i];
                            out.push((i, self.scan(path, path_to_id, &large_file)));
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
        let source = match fs::read_to_string(path.as_std_path()) {
            Ok(s) => s,
            Err(e) => {
                return Scan {
                    stage: None,
                    edges: Err(BuildError::Io {
                        path: path.to_path_buf(),
                        source: e,
                    }),
                };
            }
        };
        let _serial = (source.len() > LARGE_FILE_BYTES).then(|| large_file.lock());
        let stage = stratum_parser_ts::extract_stage(&source);
        let edges = self.edges_of(path, &source, path_to_id);
        Scan { stage, edges }
    }

    fn edges_of(
        &self,
        path: &Utf8Path,
        source: &str,
        path_to_id: &FxHashMap<Utf8PathBuf, ModuleId>,
    ) -> Result<Vec<(ModuleId, EdgeKind)>, BuildError> {
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
        Ok(data
            .imports
            .iter()
            .filter_map(|imp| {
                let target = self.resolver.resolve(path, &imp.specifier).ok()?;
                let id = path_to_id.get(&strip_windows_extended_prefix(&target))?;
                Some((*id, imp.kind))
            })
            .collect())
    }
}

const LARGE_FILE_BYTES: usize = 1 << 20;
const MAX_WORKERS: usize = 8;

struct Scan {
    stage: Option<Stage>,
    edges: Result<Vec<(ModuleId, EdgeKind)>, BuildError>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use stratum_core::ids::LayerId;

    fn tiny_ts_root() -> Utf8PathBuf {
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/tiny-ts")
            .canonicalize_utf8()
            .unwrap()
    }

    fn layers_for_tiny_ts() -> Vec<Layer> {
        vec![
            Layer {
                id: LayerId::new(0),
                name: "app".into(),
                path: PathBuf::from("src/app"),
                depends_on: vec![LayerId::new(1), LayerId::new(2), LayerId::new(3)],
            },
            Layer {
                id: LayerId::new(1),
                name: "features".into(),
                path: PathBuf::from("src/features"),
                depends_on: vec![LayerId::new(2), LayerId::new(3)],
            },
            Layer {
                id: LayerId::new(2),
                name: "entities".into(),
                path: PathBuf::from("src/entities"),
                depends_on: vec![LayerId::new(3)],
            },
            Layer {
                id: LayerId::new(3),
                name: "shared".into(),
                path: PathBuf::from("src/shared"),
                depends_on: vec![],
            },
        ]
    }

    #[test]
    fn builds_tiny_ts_graph_with_known_module_count() {
        let cfg = BuildConfig {
            project_root: tiny_ts_root(),
            layers: layers_for_tiny_ts(),
            default_stage: Stage::new(2).unwrap(),
            default_visibility: VisibilityScope::Public,
        };
        let g = GraphBuilder::new(cfg).build().unwrap();
        assert_eq!(
            g.module_count(),
            8,
            "expected 8 modules under configured layers (legacy/old-helper.js excluded)"
        );
        assert!(
            g.edge_count() >= 5,
            "expect at least 5 internal edges, got {}",
            g.edge_count()
        );
    }
}
