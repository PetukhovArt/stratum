use std::path::PathBuf;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, ConfigError};
use stratum_core::{
    ids::{LayerId, RuleId},
    stage::Stage,
    types::Layer,
    violation::Violation,
    visibility::VisibilityScope,
};
use stratum_graph::{BuildConfig, GraphBuilder};
use stratum_plugins_rhai::{LoadError as PluginLoadError, PLUGIN_ID_FLOOR};
use stratum_rules::RuleRegistry;

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
    #[error("Plugin load error: {0}")]
    Plugin(#[from] PluginLoadError),
}

/// Build the engine input (graph + config + root + registry).
///
/// # Errors
/// - [`PipelineError::Build`] if the graph cannot be assembled.
/// - [`PipelineError::Plugin`] if a referenced Rhai script fails to load or compile.
pub fn build(root: &Utf8Path, config: &Config) -> Result<EngineInput, PipelineError> {
    let root_abs = root
        .canonicalize_utf8()
        .unwrap_or_else(|_| root.to_path_buf());
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
                    let pos = config.layers.iter().position(|x| &x.id == dep).unwrap_or(0);
                    LayerId::new(u32::try_from(pos).unwrap_or(0))
                })
                .collect(),
        })
        .collect();
    let build_cfg = BuildConfig {
        project_root: root_abs.clone(),
        layers,
        default_stage: Stage::new(2).unwrap_or_else(|_| unreachable!("Stage(2) is valid")),
        default_visibility: VisibilityScope::Public,
    };
    let graph = GraphBuilder::new(build_cfg)
        .with_extractor(Box::new(stratum_parser_vue::VueExtractor::new()))
        .build()?;

    let mut registry = RuleRegistry::with_builtins();
    register_plugins(&mut registry, &root_abs, config)?;

    Ok(EngineInput {
        graph: Arc::new(graph),
        config: Arc::new(config.clone()),
        project_root: root_abs,
        registry: Arc::new(registry),
    })
}

fn register_plugins(
    registry: &mut RuleRegistry,
    root: &Utf8Path,
    config: &Config,
) -> Result<(), PipelineError> {
    let mut next_id = PLUGIN_ID_FLOOR;
    for (slug, rule_cfg) in &config.rules {
        let Some(script_rel) = rule_cfg.script.as_ref() else {
            continue;
        };
        let script = if script_rel.is_absolute() {
            script_rel.clone()
        } else {
            root.join(script_rel)
        };
        let leaked_slug: &'static str = Box::leak(slug.clone().into_boxed_str());
        let rule = stratum_plugins_rhai::load(leaked_slug, RuleId::new(next_id), &script)?;
        registry.register(rule);
        next_id = next_id.saturating_add(1);
    }
    Ok(())
}

/// Run the full pipeline, returning all violations.
///
/// # Errors
/// Returns [`PipelineError`] if the graph cannot be built or a plugin fails to load.
pub fn run(root: &Utf8Path, config: &Config) -> Result<Vec<Violation>, PipelineError> {
    let input = build(root, config)?;
    Ok(RuleEngine::run(&input))
}
