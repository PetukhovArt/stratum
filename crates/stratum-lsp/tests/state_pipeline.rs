#![allow(clippy::unwrap_used)]

use camino::Utf8PathBuf;
use stratum_engine::{config, pipeline};

fn fixture_root() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts-violations")
        .canonicalize_utf8()
        .unwrap()
}

#[test]
fn pipeline_produces_violations_for_tiny_ts_violations() {
    let root = fixture_root();
    let config = config::infer(&root);
    let input = pipeline::build(&root, &config).unwrap();
    let violations = input.violations().unwrap();
    assert!(
        !violations.is_empty(),
        "expected tiny-ts-violations to produce at least one violation"
    );
}
