use std::collections::HashMap;
use std::path::PathBuf;

use camino::Utf8Path;

use stratum_config::{Config, ConfigError};
use stratum_core::{severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::registry::RuleRegistry;
use crate::severity::{EffectiveSeverities, FileSeverities};

/// Run every built-in rule against `graph`, then cap each violation's severity
/// at the per-file effective config: a rule may downgrade a finding, never
/// upgrade it. Violations whose effective severity is `Severity::Off` are dropped.
///
/// # Errors
/// Returns [`stratum_config::ConfigError`] if any override block contains an
/// invalid glob pattern.
pub fn run_all(
    graph: &CompoundGraph,
    config: &Config,
    project_root: &Utf8Path,
) -> Result<Vec<Violation>, ConfigError> {
    run_all_with_registry(graph, config, project_root, &RuleRegistry::with_builtins())
}

/// [`run_all`] over `registry` as given: built-in rules are not added to it.
///
/// # Errors
/// Returns [`stratum_config::ConfigError`] if any override block contains an
/// invalid glob pattern.
pub fn run_all_with_registry(
    graph: &CompoundGraph,
    config: &Config,
    project_root: &Utf8Path,
    registry: &RuleRegistry,
) -> Result<Vec<Violation>, ConfigError> {
    let effective = EffectiveSeverities::new(config, project_root)?;
    let project = effective.for_project();
    let mut by_file: HashMap<PathBuf, FileSeverities<'_>> = HashMap::new();
    let mut all = Vec::new();
    for r in registry.rules() {
        if effective.off_everywhere(r.as_ref()) {
            continue;
        }
        let base_options = config
            .rules
            .get(r.slug())
            .map_or_else(|| serde_json::Value::Null, |rc| rc.options.clone());
        for mut v in r.run(graph, Severity::Error, &base_options) {
            let sev = if r.project_wide() {
                project.of(r.as_ref())
            } else {
                by_file
                    .entry(v.file.clone())
                    .or_insert_with_key(|file| effective.for_file(file))
                    .of(r.as_ref())
            };
            if sev == Severity::Off {
                continue;
            }
            v.severity = v.severity.clamp(Severity::Info, sev);
            all.push(v);
        }
    }
    all.sort_by(|a, b| {
        (a.rule.raw(), a.modules.first().map_or(0, |m| m.raw()))
            .cmp(&(b.rule.raw(), b.modules.first().map_or(0, |m| m.raw())))
    });
    Ok(all)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::ids;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use stratum_config::{LayerConfig, ProjectConfig, RuleConfig};
    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId, ModuleId},
        purity::Purity,
        types::{Layer, Module},
        visibility::VisibilityScope,
    };

    fn module(id: u32, layer_raw: u32, path: &str) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(path),
            container: ContainerId::new(layer_raw),
            layer: LayerId::new(layer_raw),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: stratum_core::types::Exports::default(),
            calls: vec![],
            imports: vec![],
            suppressions: vec![],
        }
    }

    fn build_graph_two_layers() -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        g.layers.insert(
            LayerId::new(1),
            Layer {
                id: LayerId::new(1),
                name: "features".into(),
                path: PathBuf::from("src/features"),
                depends_on: vec![LayerId::new(2)],
                slices: false,
            },
        );
        g.layers.insert(
            LayerId::new(2),
            Layer {
                id: LayerId::new(2),
                name: "shared".into(),
                path: PathBuf::from("src/shared"),
                depends_on: vec![],
                slices: false,
            },
        );
        for m in [
            module(10, 1, "src/features/cart.ts"),
            module(20, 2, "src/shared/http.ts"),
        ] {
            let node = g.deps.add_node(m.id);
            g.node_index.insert(m.id, node);
            g.modules.insert(m.id, m);
        }
        let na = g.node_index[&ModuleId::new(20)];
        let nb = g.node_index[&ModuleId::new(10)];
        g.deps.add_edge(na, nb, EdgeKind::Static);
        g
    }

    fn base_config() -> Config {
        let mut rules = BTreeMap::new();
        rules.insert(
            "stratum/no-cross-layer-import".into(),
            RuleConfig {
                severity: Severity::Error,
                options: serde_json::Value::Null,
                script: None,
            },
        );
        Config {
            version: 1,
            structure: stratum_core::structure::Structure::default(),
            project: ProjectConfig {
                extends: None,
                ignore: vec![],
                entry_points: vec![],
            },
            layers: vec![LayerConfig {
                id: "shared".into(),
                path: "src/shared".into(),
                depends_on: vec![],
                slices: false,
            }],
            rules,
            overrides: vec![],
            features: std::collections::BTreeMap::new(),
            systems: vec![],
        }
    }

    fn root() -> camino::Utf8PathBuf {
        camino::Utf8PathBuf::from("")
    }

    #[test]
    fn run_all_finds_cross_layer_violation() {
        let g = build_graph_two_layers();
        let cfg = base_config();
        let v = run_all(&g, &cfg, &root()).unwrap();
        assert!(
            v.iter()
                .any(|x| x.rule == ids::NO_CROSS_LAYER_IMPORT && x.severity == Severity::Error),
        );
    }

    #[test]
    fn run_all_drops_off_severity() {
        let g = build_graph_two_layers();
        let mut cfg = base_config();
        cfg.rules.insert(
            "stratum/no-cross-layer-import".into(),
            RuleConfig {
                severity: Severity::Off,
                options: serde_json::Value::Null,
                script: None,
            },
        );
        let v = run_all(&g, &cfg, &root()).unwrap();
        assert!(!v.iter().any(|x| x.rule == ids::NO_CROSS_LAYER_IMPORT));
    }

    struct Downgrading;

    impl crate::rule::Rule for Downgrading {
        type Scope = crate::scope::ProjectScope;
        type Options = crate::rule::EmptyOptions;

        fn id(&self) -> stratum_core::ids::RuleId {
            ids::NO_CROSS_LAYER_IMPORT
        }
        fn slug(&self) -> &'static str {
            "stratum/no-cross-layer-import"
        }
        fn default_severity(&self) -> Severity {
            Severity::Warning
        }
        fn check(
            &self,
            _graph: &CompoundGraph,
            _scope: crate::scope::ProjectScope,
            _options: &crate::rule::EmptyOptions,
            severity: Severity,
        ) -> Vec<Violation> {
            [severity, Severity::Info]
                .into_iter()
                .map(|severity| Violation {
                    rule: ids::NO_CROSS_LAYER_IMPORT,
                    severity,
                    message: String::new(),
                    file: PathBuf::from("src/a.ts"),
                    location: stratum_core::violation::SourceLocation { line: 1, column: 1 },
                    modules: vec![],
                    edge: None,
                    fix: None,
                })
                .collect()
        }
    }

    #[test]
    fn a_rule_may_downgrade_a_finding_but_never_upgrade_it() {
        let mut registry = RuleRegistry::new();
        registry.register(Downgrading);
        let severities = |configured| {
            let mut cfg = base_config();
            cfg.rules
                .get_mut("stratum/no-cross-layer-import")
                .unwrap()
                .severity = configured;
            run_all_with_registry(&CompoundGraph::empty(), &cfg, &root(), &registry)
                .unwrap()
                .into_iter()
                .map(|v| v.severity)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            severities(Severity::Error),
            [Severity::Error, Severity::Info]
        );
        assert_eq!(severities(Severity::Info), [Severity::Info, Severity::Info]);
        assert!(severities(Severity::Off).is_empty());
    }

    struct Counting(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl crate::rule::Rule for Counting {
        type Scope = crate::scope::ProjectScope;
        type Options = crate::rule::EmptyOptions;

        fn id(&self) -> stratum_core::ids::RuleId {
            ids::NO_CROSS_LAYER_IMPORT
        }
        fn slug(&self) -> &'static str {
            "stratum/no-cross-layer-import"
        }
        fn default_severity(&self) -> Severity {
            Severity::Warning
        }
        fn check(
            &self,
            graph: &CompoundGraph,
            scope: crate::scope::ProjectScope,
            options: &crate::rule::EmptyOptions,
            severity: Severity,
        ) -> Vec<Violation> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Downgrading.check(graph, scope, options, severity)
        }
    }

    #[test]
    fn a_rule_is_run_only_where_the_config_turns_it_on() {
        let runs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut registry = RuleRegistry::new();
        registry.register(Counting(runs.clone()));
        let mut cfg = base_config();
        cfg.rules
            .get_mut("stratum/no-cross-layer-import")
            .unwrap()
            .severity = Severity::Off;
        let findings = |cfg: &Config| {
            run_all_with_registry(&CompoundGraph::empty(), cfg, &root(), &registry)
                .unwrap()
                .len()
        };

        assert_eq!(findings(&cfg), 0);
        assert_eq!(runs.load(std::sync::atomic::Ordering::Relaxed), 0);

        cfg.overrides.push(stratum_config::OverrideBlock {
            files: vec!["**".into()],
            rules: [(
                "stratum/no-cross-layer-import".to_owned(),
                stratum_config::OverrideRule {
                    severity: Severity::Warning,
                },
            )]
            .into(),
        });
        assert_eq!(findings(&cfg), 2);
        assert_eq!(runs.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
