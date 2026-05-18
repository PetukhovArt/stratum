// Phase 7 exit gate: diagnostics-after-edit < 100 ms on tiny-ts.
// Measures the cost of one full pipeline rebuild + violation conversion,
// which is what `did_change` triggers after debounce.
#![allow(clippy::unwrap_used)]

use camino::Utf8PathBuf;
use criterion::{Criterion, criterion_group, criterion_main};

use stratum_lint::engine::RuleEngine;
use stratum_lint::pipeline;
use stratum_lint::zero_config;

fn fixture_root() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts-violations")
        .canonicalize_utf8()
        .unwrap()
}

fn bench_recompute_tiny_ts_violations(c: &mut Criterion) {
    let root = fixture_root();
    let config = zero_config::infer(&root);
    c.bench_function("lsp_recompute_tiny_ts_violations", |b| {
        b.iter(|| {
            let input = pipeline::build(&root, &config).unwrap();
            let violations = RuleEngine::run(&input);
            criterion::black_box(violations);
        });
    });
}

criterion_group!(benches, bench_recompute_tiny_ts_violations);
criterion_main!(benches);
