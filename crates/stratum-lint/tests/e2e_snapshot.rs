#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn snapshot_writes_graph_json() {
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
    let text = std::fs::read_to_string(&out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["version"], 1);
}
