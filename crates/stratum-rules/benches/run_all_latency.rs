// Baseline 2026-05-18 on Windows 10 Pro: ~85 µs/iter for run_all on tiny-ts-violations.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use camino::Utf8PathBuf;
use criterion::{Criterion, criterion_group, criterion_main};

use stratum_core::{ids::LayerId, purity::Purity, types::Layer, visibility::VisibilityScope};
use stratum_graph::{BuildConfig, GraphBuilder};

fn bench_run_all_tiny_ts_violations(c: &mut Criterion) {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts-violations")
        .canonicalize_utf8()
        .unwrap();
    let config = stratum_config::parse_file(&root.join("stratum.config.jsonc")).unwrap();
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
                .map(|dep_id| {
                    let pos = config
                        .layers
                        .iter()
                        .position(|x| &x.id == dep_id)
                        .unwrap_or(0);
                    LayerId::new(u32::try_from(pos).unwrap())
                })
                .collect(),
        })
        .collect();
    let cfg = BuildConfig {
        project_root: root.clone(),
        layers,
        default_purity: Purity::new(2).unwrap(),
        default_visibility: VisibilityScope::Public,
    };
    let graph = GraphBuilder::new(cfg).build().unwrap();
    c.bench_function("run_all_tiny_ts_violations", |b| {
        b.iter(|| {
            let v = stratum_rules::run_all(&graph, &config, &root).unwrap();
            criterion::black_box(v);
        });
    });
}

criterion_group!(benches, bench_run_all_tiny_ts_violations);
criterion_main!(benches);
