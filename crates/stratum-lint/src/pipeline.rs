use std::path::PathBuf;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, ConfigError};
use stratum_core::{
    ids::LayerId, stage::Stage, types::Layer, violation::Violation, visibility::VisibilityScope,
};
use stratum_graph::{BuildConfig, GraphBuilder};

use crate::engine::{EngineInput, RuleEngine};

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Could not read config at {path}: {source}")]
    #[allow(dead_code)]
    ConfigIo {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Config(#[from] ConfigError),
    #[error("Graph build error: {0}")]
    Build(#[from] stratum_graph::BuildError),
}

/// Build the engine input (graph + config + root). Callers that want to
/// invalidate individual files keep the `EngineInput` around between runs.
///
/// # Errors
/// Returns [`PipelineError::Build`] if the graph cannot be assembled.
pub fn build(root: &Utf8Path, config: &Config) -> Result<EngineInput, PipelineError> {
    let layers: Vec<Layer> = config
        .layers
        .iter()
        .enumerate()
        .map(|(i, l)| Layer {
            id: LayerId::new(u32::try_from(i).unwrap_or(u32::MAX)),
            name: l.id.clone(),
            path: PathBuf::from(l.path.to_string()),
            depends_on: l
                .depends_on
                .iter()
                .map(|dep| {
                    let pos = config
                        .layers
                        .iter()
                        .position(|x| &x.id == dep)
                        .unwrap_or(0);
                    LayerId::new(u32::try_from(pos).unwrap_or(0))
                })
                .collect(),
        })
        .collect();
    let build_cfg = BuildConfig {
        project_root: root.to_path_buf(),
        layers,
        default_stage: Stage::new(2).unwrap_or_else(|_| unreachable!("Stage(2) is valid")),
        default_visibility: VisibilityScope::Public,
    };
    let graph = GraphBuilder::new(build_cfg).build()?;
    Ok(EngineInput {
        graph: Arc::new(graph),
        config: Arc::new(config.clone()),
        project_root: root.to_path_buf(),
    })
}

/// Run the full pipeline, returning all violations.
///
/// # Errors
/// Returns [`PipelineError`] if the graph cannot be built.
pub fn run(root: &Utf8Path, config: &Config) -> Result<Vec<Violation>, PipelineError> {
    let input = build(root, config)?;
    Ok(RuleEngine::run(&input))
}
