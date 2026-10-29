use std::path::PathBuf;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, ConfigError, EntryPoints, IgnoreSet};
use stratum_core::{
    ids::{LayerId, ModuleId, RuleId},
    purity::Purity,
    types::Layer,
    violation::Violation,
};
use stratum_graph::{BuildConfig, CacheStats, CompoundGraph, GraphBuilder, Overlay, SideRole};
use stratum_plugins_rhai::{LoadError as PluginLoadError, PLUGIN_ID_FLOOR};
use stratum_rules::{DynRule, RuleRegistry};

use crate::baseline::BaselineError;
use crate::config::{ConfigSource, LoadedConfig};
use crate::docs;
use crate::features::{self, Features};
use crate::finding::{EdgeRef, Finding};
use crate::paths::{canonicalize, relative, relative_in_text};
use crate::suppression::{self, Analysis, Reported, Suppressed};
use crate::systems::{self, SystemMap};
use crate::{manifest, side};

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
    #[error(
        "`project.entry_points` glob {0:?} matches no file: an entry point is a source file, inside a layer or outside, that `project.ignore` does not ignore"
    )]
    EntryPointUnmatched(String),
    #[error("`features.{feature}.entry` glob {glob:?} matches no module")]
    FeatureEntryUnmatched { feature: String, glob: String },
}

#[derive(Debug)]
pub struct EngineInput {
    pub graph: Arc<CompoundGraph>,
    pub config: Arc<Config>,
    pub project_root: Utf8PathBuf,
    pub registry: Arc<RuleRegistry>,
    pub cache: CacheStats,
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

    pub fn analyze(&self) -> Result<Analysis, ConfigError> {
        let (findings, suppressed) = self.reported()?;
        Ok(Analysis {
            findings,
            suppressed,
        })
    }

    /// [`Self::analyze`] for a run on a diff: `changed` are the paths, relative to the root with
    /// `/`, that differ from the base. Adds the diff-mode `stratum/co-change`.
    pub fn analyze_changed(&self, changed: &[String]) -> Result<Analysis, ConfigError> {
        let mut analysis = self.analyze()?;
        analysis
            .findings
            .extend(crate::co_change::findings(self, changed)?);
        Ok(analysis)
    }

    /// `analyze`'s findings, each with the violation it came from (graph ids instead of paths).
    pub fn analyze_violations(&self) -> Result<Vec<(Violation, Finding)>, ConfigError> {
        self.reported().map(|(kept, _)| kept)
    }

    fn reported<T: Reported>(&self) -> Result<(Vec<T>, Vec<Suppressed>), ConfigError> {
        let reported = self
            .rule_violations()?
            .into_iter()
            .map(|v| T::of(self, v))
            .collect();
        suppression::apply(self, reported)
    }

    pub fn features(&self) -> Result<Features, ConfigError> {
        Features::of(&self.graph, &self.config, &self.project_root)
    }

    /// `None` without `systems` in the config. Reads the contract files of each system.
    #[must_use]
    pub fn systems(&self) -> Option<SystemMap> {
        SystemMap::of(&self.graph, &self.config, &self.project_root)
    }

    /// The directory of system `id`'s `root`, if it declares one.
    #[must_use]
    pub fn system_root(&self, id: &str) -> Option<Utf8PathBuf> {
        systems::system_root(&self.config, &self.project_root, id)
    }

    pub fn violations(&self) -> Result<Vec<Finding>, ConfigError> {
        self.analyze().map(|a| a.findings)
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

    fn fix_of(&self, rule: RuleId, slug: &str, own: Option<String>) -> String {
        own.filter(|fix| !fix.is_empty())
            .or_else(|| {
                let doc = self.registry.by_id(rule)?.doc()?;
                Some(doc.fix).filter(|fix| !fix.is_empty())
            })
            .unwrap_or_else(|| format!("Run \"stratum-lint explain {slug}\""))
    }

    #[must_use]
    pub fn finding(&self, v: Violation) -> Finding {
        let slug = self.slug_of(v.rule);
        Finding {
            fix: relative_in_text(&self.fix_of(v.rule, slug, v.fix), &self.project_root),
            rule: slug.to_owned(),
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
        }
    }
}

pub fn build(root: &Utf8Path, loaded: &LoadedConfig) -> Result<EngineInput, EngineError> {
    build_with_builder(root, loaded).map(|(input, _)| input)
}

pub fn build_with_builder(
    root: &Utf8Path,
    loaded: &LoadedConfig,
) -> Result<(EngineInput, GraphBuilder), EngineError> {
    let config = &loaded.config;
    let identity = match &loaded.source {
        ConfigSource::File(path) => canonicalize(path)
            .unwrap_or_else(|_| path.clone())
            .into_string(),
        ConfigSource::Inferred => "zero-config".to_owned(),
    };
    let (root_abs, graph, cache, builder) = graph(root, config, Source::Disk { identity })?;
    let registry = registry(&root_abs, config)?;

    Ok((
        EngineInput {
            graph: Arc::new(graph),
            cache,
            config: Arc::new(config.clone()),
            project_root: root_abs,
            registry: Arc::new(registry),
        },
        builder,
    ))
}

pub(crate) enum Source {
    Disk { identity: String },
    Overlaid(Vec<(String, Option<String>)>),
}

pub(crate) fn graph(
    root: &Utf8Path,
    config: &Config,
    source: Source,
) -> Result<(Utf8PathBuf, CompoundGraph, CacheStats, GraphBuilder), EngineError> {
    let (root_abs, mut graph, cache, builder) = graph_without_entry_points(root, config, source)?;
    let entry = EntryPoints::new(config)?;
    let mut hit = vec![false; entry.len()];
    graph.entry_points = graph
        .modules
        .values()
        .filter(|m| entry.mark(&relative(&m.path, &root_abs), &mut hit))
        .map(|m| m.id)
        .collect();
    let ignore = IgnoreSet::new(config)?;
    let walk = side::Walk::new(&root_abs, config, &ignore);
    let mut side_files: Vec<(String, SideRole)> = Vec::new();
    for base in entry.base_dirs() {
        for rel in walk.outside_layers(&base, None) {
            if entry.mark(&rel, &mut hit) {
                side_files.push((rel, SideRole::EntryPoint));
            }
        }
    }
    if let Some(i) = hit.iter().position(|h| !h) {
        return Err(EngineError::EntryPointUnmatched(entry.glob(i).to_owned()));
    }
    if let Some((feature, glob)) = features::unmatched_entry(&graph, config, &root_abs)? {
        return Err(EngineError::FeatureEntryUnmatched { feature, glob });
    }
    if side::wants_ignored(config) {
        let ignored = walk.ignored_in_layers().into_iter();
        side_files.extend(ignored.map(|rel| (rel, SideRole::Ignored)));
    }
    let listed = walk.matching(&side::listed(config)).into_iter();
    side_files.extend(listed.map(|rel| (rel, SideRole::Listed)));
    if side::enabled(config, side::UNUSED_DEPENDENCIES) {
        graph.manifest = manifest::read(&root_abs);
    }
    graph.side_files = side::scan(&builder, &graph, &root_abs, side_files);
    graph.nav_docs = docs::read(&graph, &root_abs, config);
    Ok((root_abs, graph, cache, builder))
}

pub(crate) fn graph_without_entry_points(
    root: &Utf8Path,
    config: &Config,
    source: Source,
) -> Result<(Utf8PathBuf, CompoundGraph, CacheStats, GraphBuilder), EngineError> {
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
    let mut tracked_calls = stratum_rules::tracked_calls(config);
    systems::track_calls(config, &mut tracked_calls);
    let build_cfg = BuildConfig {
        project_root: root_abs.clone(),
        layers,
        structure: config.structure.clone(),
        tracked_calls,
        default_purity: Purity::DEFAULT,
    };
    let ignore = IgnoreSet::new(config)?;
    let vue = stratum_parser_vue::VueExtractor::new(build_cfg.tracked_calls.clone());
    let mut builder = GraphBuilder::new(build_cfg)
        .with_extractor(Box::new(vue))
        .with_ignore(move |rel| ignore.is_ignored(rel));
    match source {
        Source::Disk { identity } => {
            builder =
                builder.with_cache_dir(crate::cache::dir().map(Utf8Path::to_path_buf), &identity);
        }
        Source::Overlaid(overlay) if !overlay.is_empty() => {
            let files = overlay.into_iter().map(|(rel, text)| {
                let path = rel
                    .split('/')
                    .fold(root_abs.as_std_path().to_path_buf(), |p, c| p.join(c));
                (path, text)
            });
            builder = builder.with_overlay(Overlay::new(files));
        }
        Source::Overlaid(_) => {}
    }
    let (graph, cache) = builder.build()?;
    Ok((root_abs, graph, cache, builder))
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
