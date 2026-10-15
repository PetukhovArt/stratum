use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

use camino::{Utf8Path, Utf8PathBuf};

use stratum_config::{Config, ConfigError, resolve_for_file};
use stratum_core::{severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::registry::RuleRegistry;

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
    let mut all = Vec::new();
    let mut severities_by_file: HashMap<Utf8PathBuf, BTreeMap<String, Severity>> = HashMap::new();
    for r in registry.rules() {
        let slug = r.slug();
        let default_sev = r.default_severity();
        let base_options = config
            .rules
            .get(slug)
            .map_or_else(|| serde_json::Value::Null, |rc| rc.options.clone());
        let raw = r.run(graph, Severity::Error, &base_options);
        for mut v in raw {
            let probe = if r.project_wide() {
                Utf8PathBuf::from("<project>")
            } else {
                to_relative_forward_slash(&v.file, project_root)
            };
            let severities = match severities_by_file.entry(probe) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => {
                    let severities = resolve_for_file(config, e.key())?.severities;
                    e.insert(severities)
                }
            };
            let sev = severities.get(slug).copied().unwrap_or(default_sev);
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

fn to_relative_forward_slash(file: &std::path::Path, project_root: &Utf8Path) -> Utf8PathBuf {
    let abs = Utf8PathBuf::from(file.to_string_lossy().to_string());
    let root = strip_windows_extended_prefix(project_root);
    let rel = abs.strip_prefix(&root).unwrap_or(&abs).to_owned();
    Utf8PathBuf::from(rel.as_str().replace('\\', "/"))
}

fn strip_windows_extended_prefix(p: &Utf8Path) -> Utf8PathBuf {
    let s = p.as_str();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        Utf8PathBuf::from(rest)
    } else {
        p.to_owned()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::ids;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
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
        }
    }

    fn build_graph_two_layers() -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
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
            },
            layers: vec![LayerConfig {
                id: "shared".into(),
                path: "src/shared".into(),
                depends_on: vec![],
                slices: false,
            }],
            rules,
            overrides: vec![],
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
                    suggestion: None,
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
}
