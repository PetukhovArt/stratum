#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn rhai_plugin_emits_violation_in_json_output() {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args([
            "--format",
            "json",
            "lint",
            "../../tests/fixtures/uses-legacy-plugin",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Module is under src/legacy/"));
}
