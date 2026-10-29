#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use camino::Utf8PathBuf;
use stratum_config::{Config, LayerConfig, ProjectConfig, RuleConfig};
use stratum_core::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId, RuleId},
    purity::Purity,
    severity::Severity,
    types::{Layer, Module},
    visibility::VisibilityScope,
};
use stratum_graph::CompoundGraph;
use stratum_plugins_rhai::{PLUGIN_ID_FLOOR, load};
use stratum_rules::{RuleRegistry, run_all_with_registry};

fn fixture_dir() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

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
        local_refs: vec![],
    }
}

fn build_graph() -> CompoundGraph {
    let mut g = CompoundGraph::empty();
    g.layers.insert(
        LayerId::new(1),
        Layer {
            id: LayerId::new(1),
            name: "app".into(),
            path: PathBuf::from("src/app"),
            depends_on: vec![],
            slices: false,
        },
    );
    g.layers.insert(
        LayerId::new(2),
        Layer {
            id: LayerId::new(2),
            name: "legacy".into(),
            path: PathBuf::from("src/legacy"),
            depends_on: vec![],
            slices: false,
        },
    );
    for m in [
        module(10, 1, "src/app/main.ts"),
        module(20, 2, "src/legacy/old.ts"),
    ] {
        let node = g.deps.add_node(m.id);
        g.node_index.insert(m.id, node);
        g.modules.insert(m.id, m);
    }
    let _ = g.deps.add_edge(
        g.node_index[&ModuleId::new(20)],
        g.node_index[&ModuleId::new(10)],
        EdgeKind::Static,
    );
    g
}

fn config_with_custom_rule() -> Config {
    let mut rules = BTreeMap::new();
    rules.insert(
        "custom/no-legacy-imports".into(),
        RuleConfig {
            severity: Severity::Warning,
            options: serde_json::Value::Null,
            script: Some(fixture_dir().join("no_legacy_imports.rhai")),
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
            id: "app".into(),
            path: "src/app".into(),
            depends_on: vec![],
            slices: false,
        }],
        rules,
        overrides: vec![],
        features: std::collections::BTreeMap::new(),
        systems: vec![],
    }
}

#[test]
fn rhai_rule_flags_legacy_module_via_registry() {
    let script = fixture_dir().join("no_legacy_imports.rhai");
    let rule = load(
        "custom/no-legacy-imports",
        RuleId::new(PLUGIN_ID_FLOOR),
        &script,
    )
    .unwrap();

    let mut registry = RuleRegistry::with_builtins();
    registry.register(rule);

    let g = build_graph();
    let cfg = config_with_custom_rule();
    let root = Utf8PathBuf::from("");
    let violations = run_all_with_registry(&g, &cfg, &root, &registry).unwrap();

    let custom: Vec<_> = violations
        .iter()
        .filter(|v| v.rule == RuleId::new(PLUGIN_ID_FLOOR))
        .collect();
    assert_eq!(custom.len(), 1, "expected exactly one legacy hit");
    assert!(custom[0].message.contains("legacy"));
    assert!(custom[0].fix.is_some());
}

#[test]
fn script_sees_outgoing_imports_and_rule_options() {
    use std::io::Write;

    let mut script = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        script,
        r#"fn check(m, options) {{
            let out = [];
            for i in m.imports {{
                if i.layer == options.banned {{
                    out.push(#{{ message: m.layer + " -> " + i.path + " (" + i.kind + ")" }});
                }}
            }}
            out
        }}"#
    )
    .unwrap();
    let path = Utf8PathBuf::from_path_buf(script.path().to_path_buf()).unwrap();
    let rule = load("custom/banned-layer", RuleId::new(PLUGIN_ID_FLOOR), &path).unwrap();
    let mut registry = RuleRegistry::new();
    registry.register(rule);

    let mut cfg = config_with_custom_rule();
    cfg.rules = BTreeMap::from([(
        "custom/banned-layer".into(),
        RuleConfig {
            severity: Severity::Error,
            options: serde_json::json!({ "banned": "app" }),
            script: None,
        },
    )]);
    let violations =
        run_all_with_registry(&build_graph(), &cfg, &Utf8PathBuf::from(""), &registry).unwrap();
    let messages: Vec<&str> = violations.iter().map(|v| v.message.as_str()).collect();
    assert_eq!(messages, vec!["legacy -> src/app/main.ts (static)"]);
}
