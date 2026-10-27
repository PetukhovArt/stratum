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
        "project": { "ignore": ignore },
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

fn cycle_count_with_options(options: &serde_json::Value) -> usize {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/picker.ts"),
        "import type { PickFile } from './pick-file';\nexport const picker: PickFile = () => 1;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/pick-file.ts"),
        "export type PickFile = () => number;\nexport async function pick() {\n  const { picker } = await import('./picker');\n  return picker();\n}\n",
    )
    .unwrap();
    let config = serde_json::json!({
        "version": 1,
        "layers": [{ "id": "app", "path": "src", "depends_on": [] }],
        "rules": { "stratum/no-circular-deps": { "severity": "error", "options": options } },
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
fn a_cycle_closed_only_by_a_type_import_or_a_dynamic_import_is_ignored_on_request() {
    assert_eq!(cycle_count_with_options(&serde_json::json!({})), 1);
    assert_eq!(
        cycle_count_with_options(&serde_json::json!({ "ignore_type_only": true })),
        0
    );
    assert_eq!(
        cycle_count_with_options(&serde_json::json!({ "ignore_dynamic": true })),
        0
    );
}

#[test]
fn a_mistyped_cycle_option_is_a_config_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.ts"), "export const a = 1;\n").unwrap();
    let config = serde_json::json!({
        "version": 1,
        "layers": [{ "id": "app", "path": "src", "depends_on": [] }],
        "rules": { "stratum/no-circular-deps": { "severity": "error", "options": { "ignore_type_only": 1 } } },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("--no-baseline")
        .arg(root)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("ignore_type_only"));
}

#[test]
fn explain_lists_the_cycle_options() {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["explain", "stratum/no-circular-deps"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("ignore_type_only")
                .and(predicate::str::contains("ignore_dynamic")),
        );
}

fn lint_with_entry_points(entry_points: &[&str]) -> assert_cmd::assert::Assert {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src/app")).unwrap();
    std::fs::write(root.join("src/app/main.ts"), "export const main = 1;\n").unwrap();
    std::fs::write(root.join("src/boot.ts"), "import './app/main';\n").unwrap();
    let config = serde_json::json!({
        "version": 1,
        "project": { "entry_points": entry_points },
        "layers": [{ "id": "app", "path": "src/app", "depends_on": [] }],
        "rules": { "stratum/unreachable-modules": { "severity": "error" } },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("--no-baseline")
        .arg(root)
        .assert()
}

#[test]
fn an_entry_glob_matching_no_file_exits_2() {
    lint_with_entry_points(&["src/app/main.ts"]).success();
    lint_with_entry_points(&["src/app/main.ts", "src/missing.ts"])
        .code(2)
        .stderr(predicate::str::contains(
            "\"src/missing.ts\" matches no file",
        ));
}

#[test]
fn an_entry_point_outside_every_layer_reaches_what_it_imports() {
    lint_with_entry_points(&["src/boot.ts"]).success();
    lint_with_entry_points(&["src/*.ts"]).success();
}

#[test]
fn an_ignored_entry_point_outside_the_layers_matches_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src/app")).unwrap();
    std::fs::write(root.join("src/app/main.ts"), "export const main = 1;\n").unwrap();
    std::fs::write(root.join("src/boot.ts"), "import './app/main';\n").unwrap();
    let config = serde_json::json!({
        "version": 1,
        "project": { "entry_points": ["src/boot.ts"], "ignore": ["src/boot.ts"] },
        "layers": [{ "id": "app", "path": "src/app", "depends_on": [] }],
        "rules": { "stratum/unreachable-modules": { "severity": "error" } },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("--no-baseline")
        .arg(root)
        .assert()
        .code(2);
}

#[test]
fn names_an_entry_point_outside_the_layers_imports_are_used() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src/app")).unwrap();
    std::fs::write(
        root.join("src/app/app.ts"),
        "export const used = 1;\nexport const unused = 2;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/main.ts"),
        "import { used } from './app/app';\nused;\n",
    )
    .unwrap();
    let config = serde_json::json!({
        "version": 1,
        "project": { "entry_points": ["src/main.ts"] },
        "layers": [{ "id": "app", "path": "src/app", "depends_on": [] }],
        "rules": {
            "stratum/unused-exports": { "severity": "error" },
            "stratum/unreachable-modules": { "severity": "error" },
        },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["--format", "json", "--no-baseline"])
        .arg(root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let found: Vec<(&str, &str)> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (v["rule"].as_str().unwrap(), v["message"].as_str().unwrap()))
        .filter(|(rule, _)| rule.starts_with("stratum/un"))
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "stratum/unused-exports");
    assert!(found[0].1.contains("`unused`"), "{found:?}");
}
