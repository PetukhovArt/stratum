#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use camino::Utf8PathBuf;
use criterion::{Criterion, criterion_group, criterion_main};

use stratum_config::{Config, LayerConfig, ProjectConfig, RuleConfig};
use stratum_core::{ids::LayerId, purity::Purity, severity::Severity, types::Layer};
use stratum_graph::{BuildConfig, GraphBuilder};

fn tiny_ts_config() -> Config {
    let mut rules = BTreeMap::new();
    for slug in [
        "stratum/no-cross-layer-import",
        "stratum/no-circular-deps",
        "stratum/purity",
        "stratum/miller-limit",
        "stratum/depth-ratio",
    ] {
        rules.insert(
            slug.into(),
            RuleConfig {
                severity: Severity::Warning,
                options: serde_json::Value::Null,
                script: None,
            },
        );
    }
    Config {
        version: 1,
        structure: stratum_core::structure::Structure::default(),
        project: ProjectConfig {
            extends: None,
            ignore: vec![],
            entry_points: vec![],
        },
        layers: vec![
            LayerConfig {
                id: "app".into(),
                path: "src/app".into(),
                depends_on: vec!["features".into(), "entities".into(), "shared".into()],
                slices: false,
            },
            LayerConfig {
                id: "features".into(),
                path: "src/features".into(),
                depends_on: vec!["entities".into(), "shared".into()],
                slices: false,
            },
            LayerConfig {
                id: "entities".into(),
                path: "src/entities".into(),
                depends_on: vec!["shared".into()],
                slices: false,
            },
            LayerConfig {
                id: "shared".into(),
                path: "src/shared".into(),
                depends_on: vec![],
                slices: false,
            },
        ],
        rules,
        overrides: vec![],
    }
}

fn bench_cold_lint_tiny_ts(c: &mut Criterion) {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts")
        .canonicalize_utf8()
        .unwrap();
    let config = tiny_ts_config();
    c.bench_function("cold_lint_tiny_ts", |b| {
        b.iter(|| {
            let layers: Vec<Layer> = config
                .layers
                .iter()
                .enumerate()
                .map(|(i, l)| Layer {
                    id: LayerId::new(u32::try_from(i).unwrap()),
                    name: l.id.clone(),
                    path: PathBuf::from(l.path.to_string()),
                    depends_on: l
                        .depends_on
                        .iter()
                        .map(|d| {
                            let pos = config.layers.iter().position(|x| &x.id == d).unwrap_or(0);
                            LayerId::new(u32::try_from(pos).unwrap())
                        })
                        .collect(),
                    slices: false,
                })
                .collect();
            let cfg = BuildConfig {
                project_root: root.clone(),
                layers,
                default_purity: Purity::new(2).unwrap(),
                structure: stratum_core::structure::Structure::default(),
                tracked_calls: std::collections::HashSet::new(),
            };
            let graph = GraphBuilder::new(cfg).build().unwrap();
            let g = Arc::new(graph);
            let violations = stratum_rules::run_all(&g, &config, &root).unwrap();
            criterion::black_box(violations);
        });
    });
}

criterion_group!(benches, bench_cold_lint_tiny_ts);
criterion_main!(benches);
