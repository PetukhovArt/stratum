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
    assert_eq!(parsed["version"], 2);
    assert_eq!(parsed["summary"]["errors"], 3);
    let v = &parsed["violations"][0];
    assert!(v["rule"].as_str().unwrap().starts_with("stratum/"));
    assert!(v["file"].as_str().unwrap().starts_with("src/"));
}

#[test]
fn bad_config_is_a_tool_failure() {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args([
            "--config",
            "does-not-exist.jsonc",
            "../../tests/fixtures/tiny-ts-violations",
        ])
        .assert()
        .code(2);
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
    let result = &parsed["runs"][0]["results"][0];
    assert!(result["ruleId"].as_str().unwrap().starts_with("stratum/"));
    let uri = result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
        .as_str()
        .unwrap();
    assert!(uri.starts_with("src/"), "uri is ROOT-relative: {uri}");
}
