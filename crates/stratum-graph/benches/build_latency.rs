// Baseline 2026-05-18 on i5-12600K: ~2.8 ms/iter for 8 modules + 7 edges.
// PRD success metric: cold lint 10K files < 5s — extrapolation, not gated here.

use std::path::PathBuf;

use camino::Utf8PathBuf;
use criterion::{Criterion, criterion_group, criterion_main};

use stratum_core::{ids::LayerId, purity::Purity, types::Layer, visibility::VisibilityScope};
use stratum_graph::{BuildConfig, GraphBuilder};

#[allow(clippy::unwrap_used)]
fn bench_build_tiny_ts(c: &mut Criterion) {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts")
        .canonicalize_utf8()
        .unwrap();
    let cfg = BuildConfig {
        project_root: root,
        layers: vec![
            Layer {
                id: LayerId::new(0),
                name: "app".into(),
                path: PathBuf::from("src/app"),
                depends_on: vec![],
            },
            Layer {
                id: LayerId::new(1),
                name: "features".into(),
                path: PathBuf::from("src/features"),
                depends_on: vec![],
            },
            Layer {
                id: LayerId::new(2),
                name: "entities".into(),
                path: PathBuf::from("src/entities"),
                depends_on: vec![],
            },
            Layer {
                id: LayerId::new(3),
                name: "shared".into(),
                path: PathBuf::from("src/shared"),
                depends_on: vec![],
            },
        ],
        default_purity: Purity::new(2).unwrap(),
        default_visibility: VisibilityScope::Public,
    };
    c.bench_function("build_tiny_ts_cold", |b| {
        b.iter(|| {
            let g = GraphBuilder::new(cfg.clone()).build().unwrap();
            criterion::black_box(g);
        });
    });
}

criterion_group!(benches, bench_build_tiny_ts);
criterion_main!(benches);
