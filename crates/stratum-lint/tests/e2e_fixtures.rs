#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use assert_cmd::Command;

fn lint_json(fixture: &str) -> serde_json::Value {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args([
            "--format",
            "json",
            "--no-baseline",
            &format!("../../tests/fixtures/{fixture}"),
        ])
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

fn histogram(report: &serde_json::Value) -> BTreeMap<String, usize> {
    let mut by_rule = BTreeMap::new();
    for v in report["violations"].as_array().unwrap() {
        *by_rule
            .entry(v["rule"].as_str().unwrap().to_string())
            .or_insert(0) += 1;
    }
    by_rule
}

#[test]
fn tiny_ts_violations_fire_each_planted_rule() {
    let report = lint_json("tiny-ts-violations");
    let by_rule = histogram(&report);
    assert_eq!(
        by_rule.get("stratum/no-circular-deps"),
        Some(&1),
        "{by_rule:?}"
    );
    assert_eq!(
        by_rule.get("stratum/no-cross-layer-import"),
        Some(&1),
        "{by_rule:?}"
    );
    assert!(
        by_rule.get("stratum/purity").copied().unwrap_or(0) >= 1,
        "{by_rule:?}"
    );

    let xlayer = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["rule"] == "stratum/no-cross-layer-import")
        .unwrap();
    assert_eq!(
        xlayer["severity"], "warning",
        "the override under src/shared/api/** demotes no-cross-layer-import to warning"
    );
}

#[test]
fn exemplar_violation_histogram_is_stable() {
    let by_rule = histogram(&lint_json("exemplar-stratum-app"));
    insta::assert_yaml_snapshot!("exemplar_violation_histogram", by_rule);
}
