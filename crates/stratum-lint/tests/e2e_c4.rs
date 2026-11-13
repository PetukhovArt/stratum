#![allow(clippy::unwrap_used)]

use assert_cmd::Command;

const SYSTEMS: &str = "../../tests/fixtures/tiny-systems/web";
const ZONES: &str = "../../tests/fixtures/tiny-systems/web/zones.config.jsonc";
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
fn every_level_of_a_project_with_systems_is_written() {
    for level in ["system", "container", "component"] {
        let output = c4(&[SYSTEMS, "--level", level, "--container", "shared"]);
        insta::assert_snapshot!(format!("systems_mermaid_{level}"), output);
    }
}

#[test]
fn the_system_level_draws_actors_external_systems_declared_relations_and_via_modules() {
    let output = c4(&[SYSTEMS, "--config", ZONES, "--level", "system"]);
    for line in [
        r#"Person(act_operator, "Operator", "Places orders")"#,
        r#"Person(act_auditor, "auditor", "")"#,
        r#"System(sys_ui, "Web UI", "Browser client, 8 modules")"#,
        r#"System(sys_billing, "billing", "Invoices")"#,
        r#"System_Ext(sys_opc_gateway, "OPC UA gateway", "Plant signals")"#,
        r#"Rel(act_operator, sys_ui, "places orders")"#,
        r#"Rel(act_auditor, sys_billing, "reviews invoices")"#,
        r#"Rel(sys_ui, sys_api, "signs in, 4 contracts, 1 call not attributed")"#,
        r#"Rel(sys_api, sys_billing, "charges orders")"#,
        r#"Rel(sys_billing, sys_opc_gateway, "reads meter signals")"#,
        r#"Rel(sys_ui, sys_opc_gateway, "via 1 module")"#,
    ] {
        assert!(output.lines().any(|l| l == line), "{line}\n{output}");
    }
    insta::assert_snapshot!("systems_mermaid_system_zones", output);
    let files = c4(&[
        SYSTEMS,
        "--config",
        ZONES,
        "--level",
        "system",
        "--feature",
        "files",
    ]);
    assert!(!files.contains("via 1 module"), "{files}");
}

#[test]
fn with_a_feature_the_system_level_counts_only_the_contracts_it_runs() {
    for feature in ["files", "checkout"] {
        let output = c4(&[SYSTEMS, "--level", "system", "--feature", feature]);
        insta::assert_snapshot!(format!("systems_mermaid_system_{feature}"), output);
    }
}

#[test]
fn the_feature_filter_keeps_only_the_containers_and_imports_of_the_chosen_features() {
    let all = c4(&[TINY, "--config", FEATURES]);
    insta::assert_snapshot!("tiny_mermaid_container", all);
    let cart = c4(&[TINY, "--config", FEATURES, "--feature", "cart"]);
    insta::assert_snapshot!("tiny_mermaid_container_cart", cart);
    let shared = c4(&[
        TINY,
        "--config",
        FEATURES,
        "--level",
        "component",
        "--container",
        "shared",
        "--feature",
        "boot",
    ]);
    insta::assert_snapshot!("tiny_mermaid_component_shared_boot", shared);
}

#[test]
fn a_request_the_project_cannot_answer_exits_two_and_says_what_is_known() {
    let cases: [(&[&str], &str); 4] = [
        (
            &[TINY, "--level", "system"],
            "the system level needs `systems` in the config",
        ),
        (
            &[TINY, "--config", FEATURES, "--feature", "chekout"],
            r#"unknown feature "chekout"; known: boot, cart, legacy"#,
        ),
        (
            &[SYSTEMS, "--level", "component"],
            "the component level needs `--container NAME`; containers: features, shared",
        ),
        (
            &[SYSTEMS, "--level", "component", "--container", "api"],
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
