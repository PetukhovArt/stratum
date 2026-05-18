//! Pure orchestration over [`RuleRegistry`].
//!
//! Phase 3 limitation: per-file *options* overrides are not honoured — only
//! per-file *severity* is. Phase 4's CLI replaces this with per-rule Salsa
//! queries keyed on `(rule_id, module_id, config_hash)`.

use camino::Utf8PathBuf;

use stratum_config::{Config, ConfigError, resolve_for_file};
use stratum_core::{severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::ids;
use crate::registry::RuleRegistry;

/// Run every registered rule against `graph`, then re-stamp each violation's
/// severity from the per-file effective config. Violations whose effective
/// severity is `Severity::Off` are dropped.
///
/// Project-scoped rules (`no-circular-deps`) probe override resolution with the
/// sentinel path `<project>` so file globs cannot disable them.
///
/// # Errors
/// Returns [`stratum_config::ConfigError`] if any override block contains an
/// invalid glob pattern.
pub fn run_all(graph: &CompoundGraph, config: &Config) -> Result<Vec<Violation>, ConfigError> {
    let registry = RuleRegistry::with_builtins();
    let mut all = Vec::new();
    for r in registry.rules() {
        let slug = r.slug();
        let default_sev = r.default_severity();
        let base_options = config
            .rules
            .get(slug)
            .map_or_else(|| serde_json::Value::Null, |rc| rc.options.clone());
        let raw = r.run(graph, default_sev, &base_options);
        for mut v in raw {
            let probe = if r.id() == ids::NO_CIRCULAR_DEPS {
                Utf8PathBuf::from("<project>")
            } else {
                Utf8PathBuf::from(v.file.to_string_lossy().to_string())
            };
            let eff = resolve_for_file(config, &probe)?;
            let sev = eff.rules.get(slug).map_or(default_sev, |rc| rc.severity);
            if sev == Severity::Off {
                continue;
            }
            v.severity = sev;
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
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
    use stratum_config::{LayerConfig, ProjectConfig, RuleConfig};
    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId, ModuleId},
        stage::Stage,
        types::{Layer, Module},
        visibility::VisibilityScope,
    };

    fn module(id: u32, layer_raw: u32, path: &str) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(path),
            container: ContainerId::new(layer_raw),
            layer: LayerId::new(layer_raw),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
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
            },
        );
        g.layers.insert(
            LayerId::new(2),
            Layer {
                id: LayerId::new(2),
                name: "shared".into(),
                path: PathBuf::from("src/shared"),
                depends_on: vec![],
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
            project: ProjectConfig {
                root: "./src".into(),
                extends: None,
            },
            layers: vec![LayerConfig {
                id: "shared".into(),
                path: "src/shared".into(),
                depends_on: vec![],
            }],
            rules,
            overrides: vec![],
        }
    }

    #[test]
    fn run_all_finds_cross_layer_violation() {
        let g = build_graph_two_layers();
        let cfg = base_config();
        let v = run_all(&g, &cfg).unwrap();
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
        let v = run_all(&g, &cfg).unwrap();
        assert!(!v.iter().any(|x| x.rule == ids::NO_CROSS_LAYER_IMPORT));
    }
}
