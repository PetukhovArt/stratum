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

fn cycle_count_with_ignore(ignore: &[&str]) -> usize {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src/app")).unwrap();
    std::fs::create_dir_all(root.join("src/generated")).unwrap();
    std::fs::write(
        root.join("src/app/x.ts"),
        "import { y } from '../generated/y';\nexport const x = y;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/generated/y.ts"),
        "import { x } from '../app/x';\nexport const y = x;\n",
    )
    .unwrap();
    let config = serde_json::json!({
        "version": 1,
        "project": { "root": "./src", "ignore": ignore },
        "layers": [{ "id": "app", "path": "src", "depends_on": [] }],
        "rules": { "stratum/no-circular-deps": { "severity": "error" } },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["--format", "json", "--no-baseline"])
        .arg(root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["rule"] == "stratum/no-circular-deps")
        .count()
}

#[test]
fn a_cycle_through_an_ignored_file_is_not_reported() {
    assert_eq!(cycle_count_with_ignore(&[]), 1);
    assert_eq!(cycle_count_with_ignore(&["src/generated"]), 0);
}
