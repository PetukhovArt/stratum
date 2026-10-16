#![allow(clippy::unwrap_used)]

use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

const FRONTEND_FIXTURE: &str =
    "../../frontend/stratum-visualizer-frontend/tests/fixtures/tiny-ts-snapshot.json";

#[test]
fn frontend_fixture_is_the_tiny_ts_snapshot() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("snap.json");
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args([
            "snapshot",
            "../../tests/fixtures/tiny-ts",
            "--out",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();
    let fresh = std::fs::read_to_string(&out).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(FRONTEND_FIXTURE);
    if std::env::var("INSTA_UPDATE").is_ok_and(|v| v == "always") {
        std::fs::write(&fixture, &fresh).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(&fixture)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        committed == fresh,
        "{FRONTEND_FIXTURE} is stale; regenerate with INSTA_UPDATE=always cargo test -p stratum-lint --test e2e_snapshot"
    );
}
