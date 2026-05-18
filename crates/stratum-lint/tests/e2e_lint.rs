#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn lint_clean_project_exits_zero() {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("../../tests/fixtures/tiny-ts")
        .assert()
        .success();
}

#[test]
fn lint_dirty_project_exits_one() {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("../../tests/fixtures/tiny-ts-violations")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("error"));
}

#[test]
fn json_format_is_valid_json() {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args([
            "--format",
            "json",
            "../../tests/fixtures/tiny-ts-violations",
        ])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed["version"], 1);
    assert!(parsed["violations"].is_array());
}

#[test]
fn sarif_format_has_required_fields() {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args([
            "--format",
            "sarif",
            "../../tests/fixtures/tiny-ts-violations",
        ])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed["version"], "2.1.0");
    assert_eq!(parsed["runs"][0]["tool"]["driver"]["name"], "stratum-lint");
}
