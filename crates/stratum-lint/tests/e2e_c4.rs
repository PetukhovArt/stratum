#![allow(clippy::unwrap_used)]

use assert_cmd::Command;

const SYSTEMS: &str = "../../tests/fixtures/tiny-systems/web";
const TINY: &str = "../../tests/fixtures/tiny-ts";
const FEATURES: &str = "../../tests/fixtures/tiny-ts/features.config.jsonc";

fn c4(args: &[&str]) -> String {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("c4")
        .args(args)
        .assert()
        .success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn every_format_writes_every_level_of_a_project_with_systems() {
    for format in ["mermaid", "structurizr", "plantuml"] {
        for level in ["system", "container", "component"] {
            let output = c4(&[
                SYSTEMS,
                "--format",
                format,
                "--level",
                level,
                "--container",
                "shared",
            ]);
            insta::assert_snapshot!(format!("systems_{format}_{level}"), output);
        }
    }
}

#[test]
fn with_a_feature_the_system_level_counts_only_the_contracts_it_runs() {
    for feature in ["files", "checkout"] {
        let output = c4(&[
            SYSTEMS,
            "--format",
            "mermaid",
            "--level",
            "system",
            "--feature",
            feature,
        ]);
        insta::assert_snapshot!(format!("systems_mermaid_system_{feature}"), output);
    }
}

#[test]
fn the_feature_filter_keeps_only_the_containers_and_imports_of_the_chosen_features() {
    let all = c4(&[TINY, "--config", FEATURES, "--format", "mermaid"]);
    insta::assert_snapshot!("tiny_mermaid_container", all);
    let cart = c4(&[
        TINY,
        "--config",
        FEATURES,
        "--format",
        "mermaid",
        "--feature",
        "cart",
    ]);
    insta::assert_snapshot!("tiny_mermaid_container_cart", cart);
    let shared = c4(&[
        TINY,
        "--config",
        FEATURES,
        "--format",
        "structurizr",
        "--level",
        "component",
        "--container",
        "shared",
        "--feature",
        "boot",
    ]);
    insta::assert_snapshot!("tiny_structurizr_component_shared_boot", shared);
}

#[test]
fn a_request_the_project_cannot_answer_exits_two_and_says_what_is_known() {
    let cases: [(&[&str], &str); 4] = [
        (
            &[TINY, "--format", "mermaid", "--level", "system"],
            "the system level needs `systems` in the config",
        ),
        (
            &[
                TINY,
                "--config",
                FEATURES,
                "--format",
                "mermaid",
                "--feature",
                "chekout",
            ],
            r#"unknown feature "chekout"; known: boot, cart, legacy"#,
        ),
        (
            &[SYSTEMS, "--format", "plantuml", "--level", "component"],
            "the component level needs `--container NAME`; containers: features, shared",
        ),
        (
            &[
                SYSTEMS,
                "--format",
                "plantuml",
                "--level",
                "component",
                "--container",
                "api",
            ],
            r#"unknown container "api"; known: features, shared"#,
        ),
    ];
    for (args, message) in cases {
        let out = Command::cargo_bin("stratum-lint")
            .unwrap()
            .arg("c4")
            .args(args)
            .assert()
            .code(2);
        let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
        let unwrapped = |s: &str| -> String {
            s.lines()
                .map(|l| l.trim_start_matches([' ', '|', 'x']))
                .collect::<String>()
                .split_whitespace()
                .collect()
        };
        assert!(
            unwrapped(&stderr).contains(&unwrapped(message)),
            "{args:?}: {stderr}"
        );
    }
}
