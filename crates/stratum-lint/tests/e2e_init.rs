#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn init_writes_config_file() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/app")).unwrap();
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["init", dir.path().to_str().unwrap()])
        .assert()
        .success();
    let path = dir.path().join("stratum.config.jsonc");
    assert!(path.exists());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"$schema\""));
}

#[test]
fn init_refuses_to_overwrite() {
    let dir = tempdir().unwrap();
    let cfg = dir.path().join("stratum.config.jsonc");
    std::fs::write(&cfg, "{}").unwrap();
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["init", dir.path().to_str().unwrap()])
        .assert()
        .failure();
}
