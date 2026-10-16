use std::path::PathBuf;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, ConfigError, IgnoreSet};
use stratum_core::{
    ids::{LayerId, ModuleId, RuleId},
    purity::Purity,
    types::Layer,
    violation::Violation,
};
use stratum_graph::{BuildConfig, CompoundGraph, GraphBuilder, Overlay};
use stratum_plugins_rhai::{LoadError as PluginLoadError, PLUGIN_ID_FLOOR};
use stratum_rules::{DynRule, RuleRegistry};

use crate::baseline::BaselineError;
use crate::finding::{EdgeRef, Finding};
use crate::paths::{canonicalize, relative, relative_in_text};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("{0}")]
    Config(#[from] ConfigError),
    #[error("Graph build error: {0}")]
    Build(#[from] stratum_graph::BuildError),
    #[error("Plugin load error: {0}")]
    Plugin(#[from] PluginLoadError),
    #[error("{0}")]
    Baseline(#[from] BaselineError),
}

#[derive(Debug)]
pub struct EngineInput {
    pub graph: Arc<CompoundGraph>,
    pub config: Arc<Config>,
    pub project_root: Utf8PathBuf,
    pub registry: Arc<RuleRegistry>,
}

impl EngineInput {
    pub fn rule_violations(&self) -> Result<Vec<Violation>, ConfigError> {
        stratum_rules::run_all_with_registry(
            &self.graph,
            &self.config,
            &self.project_root,
            &self.registry,
        )
    }

    pub fn violations(&self) -> Result<Vec<Finding>, ConfigError> {
        Ok(self
            .rule_violations()?
            .into_iter()
            .map(|v| self.finding(v))
            .collect())
    }

    #[must_use]
    pub fn slug_of(&self, rule: RuleId) -> &str {
        self.registry.by_id(rule).map_or("unknown", DynRule::slug)
    }

    #[must_use]
    pub fn relative(&self, path: &std::path::Path) -> String {
        relative(path, &self.project_root)
    }

    fn module_path(&self, id: ModuleId) -> String {
        self.graph
            .modules
            .get(&id)
            .map_or_else(|| format!("<module {id:?}>"), |m| self.relative(&m.path))
    }

    #[must_use]
    pub fn finding(&self, v: Violation) -> Finding {
        Finding {
            rule: self.slug_of(v.rule).to_owned(),
            severity: v.severity,
            file: self.relative(&v.file),
            location: v.location,
            edge: v.edge.map(|e| EdgeRef {
                from: self.module_path(e.from),
                to: self.module_path(e.to),
                kind: e.kind,
            }),
            members: v.modules.iter().map(|&id| self.module_path(id)).collect(),
            message: v.message,
            suggestion: v
                .suggestion
                .map(|fix| relative_in_text(&fix, &self.project_root)),
        }
    }
}

pub fn build(root: &Utf8Path, config: &Config) -> Result<EngineInput, EngineError> {
    build_with_builder(root, config).map(|(input, _)| input)
}

pub fn build_with_builder(
    root: &Utf8Path,
    config: &Config,
) -> Result<(EngineInput, GraphBuilder), EngineError> {
    let (root_abs, graph, builder) = graph(root, config, Vec::new())?;
    let registry = registry(&root_abs, config)?;

    Ok((
        EngineInput {
            graph: Arc::new(graph),
            config: Arc::new(config.clone()),
            project_root: root_abs,
            registry: Arc::new(registry),
        },
        builder,
    ))
}

pub(crate) fn graph(
    root: &Utf8Path,
    config: &Config,
    overlay: Vec<(String, Option<String>)>,
) -> Result<(Utf8PathBuf, CompoundGraph, GraphBuilder), EngineError> {
    let root_abs = canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
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
            slices: l.slices,
        })
        .collect();
    let build_cfg = BuildConfig {
        project_root: root_abs.clone(),
        layers,
        structure: config.structure.clone(),
        tracked_calls: stratum_rules::tracked_calls(config),
        default_purity: Purity::DEFAULT,
    };
    let ignore = IgnoreSet::new(config)?;
    let vue = stratum_parser_vue::VueExtractor::new(build_cfg.tracked_calls.clone());
    let mut builder = GraphBuilder::new(build_cfg)
        .with_extractor(Box::new(vue))
        .with_ignore(move |rel| ignore.is_ignored(rel));
    if !overlay.is_empty() {
        let files = overlay.into_iter().map(|(rel, text)| {
            let path = rel
                .split('/')
                .fold(root_abs.as_std_path().to_path_buf(), |p, c| p.join(c));
            (path, text)
        });
        builder = builder.with_overlay(Overlay::new(files));
    }
    let graph = builder.build()?;
    Ok((root_abs, graph, builder))
}

pub fn registry(root: &Utf8Path, config: &Config) -> Result<RuleRegistry, EngineError> {
    let mut registry = RuleRegistry::with_builtins();
    let mut next_id = PLUGIN_ID_FLOOR;
    for (slug, script) in plugin_scripts(root, config) {
        let leaked_slug: &'static str = Box::leak(slug.to_owned().into_boxed_str());
        let rule = stratum_plugins_rhai::load(leaked_slug, RuleId::new(next_id), &script)?;
        registry.register(rule);
        next_id = next_id.saturating_add(1);
    }
    registry.validate(config)?;
    Ok(registry)
}

#[must_use]
pub fn plugin_scripts<'c>(root: &Utf8Path, config: &'c Config) -> Vec<(&'c str, Utf8PathBuf)> {
    config
        .rules
        .iter()
        .filter_map(|(slug, rc)| Some((slug.as_str(), root.join(rc.script.as_ref()?))))
        .collect()
}
