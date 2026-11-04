#![allow(clippy::unwrap_used)]

use assert_cmd::Command;

const TINY: &str = "../../tests/fixtures/tiny-ts";
const CONFIG: &str = "../../tests/fixtures/tiny-ts/features.config.jsonc";

fn features(format: &str) -> String {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["features", TINY, "--config", CONFIG, "--format", format])
        .assert()
        .success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn features_json_lists_module_paths_per_feature_and_the_shared_ones() {
    insta::assert_snapshot!(features("json"));
}

#[test]
fn features_text_counts_modules_and_exclusive_ones_and_names_the_users_of_each_shared_module() {
    insta::assert_snapshot!(features("text"));
}

#[test]
fn visualize_with_an_unknown_feature_exits_two_and_lists_the_known_ones() {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args([
            "visualize",
            TINY,
            "--config",
            CONFIG,
            "--feature",
            "cart",
            "--feature",
            "chekout",
            "--no-open",
        ])
        .assert()
        .code(2);
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains(r#"unknown feature "chekout"; known: boot, cart, legacy"#),
        "{stderr}"
    );
}
