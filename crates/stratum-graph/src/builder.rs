use std::fs;
use std::path::PathBuf;

use camino::{Utf8Path, Utf8PathBuf};
use rustc_hash::FxHashMap;
use walkdir::WalkDir;

use stratum_core::ids::{ContainerId, ModuleId};
use stratum_core::stage::Stage;
use stratum_core::types::{Container, Layer, Module};
use stratum_core::visibility::VisibilityScope;
use stratum_parser_ts::{LanguageExtractor, OxcTsExtractor, PathResolver};

use crate::graph::CompoundGraph;
use crate::layer_assignment::assign_layer;

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
}

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
        }
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

        let mut next_module_id: u32 = 0;
        let mut path_to_id: FxHashMap<Utf8PathBuf, ModuleId> = FxHashMap::default();
        let mut id_to_path: Vec<(ModuleId, Utf8PathBuf)> = Vec::new();

        let mut entries: Vec<_> = WalkDir::new(root.as_std_path())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .collect();
        entries.sort_by_key(|e| e.path().to_path_buf());

        for entry in entries {
            let Some(raw) = Utf8PathBuf::from_path_buf(entry.path().to_path_buf()).ok() else {
                continue;
            };
            let path = strip_windows_extended_prefix(&raw);
            let Some(extractor) = self.pick_extractor(path.as_ref()) else {
                continue;
            };
            let _ = extractor;
            let stripped_root = strip_windows_extended_prefix(root);
            let Ok(rel) = path.strip_prefix(&stripped_root) else {
                continue;
            };
            let Some(layer) = assign_layer(&self.config.layers, rel) else {
                continue;
            };

            let id = ModuleId::new(next_module_id);
            next_module_id = next_module_id.wrapping_add(1);

            let annotated_stage = std::fs::read_to_string(path.as_std_path())
                .ok()
                .and_then(|s| stratum_parser_ts::extract_stage(&s));

            let module = Module {
                id,
                path: PathBuf::from(path.as_str()),
                container: ContainerId::new(layer.raw()),
                layer,
                stage: annotated_stage.unwrap_or(self.config.default_stage),
                visibility: self.config.default_visibility.clone(),
            };
            let node = graph.deps.add_node(id);
            graph.node_index.insert(id, node);
            graph.modules.insert(id, module);
            path_to_id.insert(path.clone(), id);
            id_to_path.push((id, path));
        }

        for (from_id, from_path) in &id_to_path {
            let source =
                fs::read_to_string(from_path.as_std_path()).map_err(|e| BuildError::Io {
                    path: from_path.clone(),
                    source: e,
                })?;
            let extractor =
                self.pick_extractor(from_path.as_ref())
                    .ok_or_else(|| BuildError::Parser {
                        path: from_path.clone(),
                        source: stratum_parser_ts::ExtractError::UnsupportedExtension(
                            from_path.clone(),
                        ),
                    })?;
            let data = extractor
                .extract(from_path.as_ref(), &source)
                .map_err(|e| BuildError::Parser {
                    path: from_path.clone(),
                    source: e,
                })?;
            for imp in &data.imports {
                let Ok(target_raw) = self.resolver.resolve(from_path.as_ref(), &imp.specifier)
                else {
                    continue;
                };
                let target_path = strip_windows_extended_prefix(&target_raw);
                let Some(to_id) = path_to_id.get(&target_path).copied() else {
                    continue;
                };
                let from_node = graph.node_index[from_id];
                let to_node = graph.node_index[&to_id];
                graph.deps.add_edge(from_node, to_node, imp.kind);
            }
        }

        Ok(graph)
    }
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
