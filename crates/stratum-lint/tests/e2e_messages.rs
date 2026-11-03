#![allow(clippy::unwrap_used)]

mod support;

use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use stratum_core::text::Key;
use stratum_rules::RuleRegistry;
use support::{PROJECT_SUBDIR, repo_with_committed_fixture_in_subdir};

const MARK: &str = "@@ ";

fn stratum(root: &Path, args: &[&str]) -> std::process::Output {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(args)
        .arg(root)
        .output()
        .unwrap()
}

fn json(root: &Path, args: &[&str]) -> Value {
    serde_json::from_slice(&stratum(root, args).stdout).unwrap()
}

fn title(slug: &str) -> String {
    format!("Rule {slug}")
}

/// A title for every built-in rule and every template marked, placeholders kept.
fn messages() -> Value {
    let titles: serde_json::Map<String, Value> = RuleRegistry::with_builtins()
        .rules()
        .iter()
        .map(|r| (r.slug().to_owned(), Value::from(title(r.slug()))))
        .collect();
    let texts: serde_json::Map<String, Value> = Key::ALL
        .iter()
        .map(|k| {
            (
                k.name().to_owned(),
                Value::from(format!("{MARK}{}", k.template())),
            )
        })
        .collect();
    serde_json::json!({ "titles": titles, "texts": texts })
}

fn with_messages(root: &Path, messages: &Value) {
    let config = root.join("stratum.config.jsonc");
    let text = std::fs::read_to_string(&config).unwrap();
    let block = format!("\"version\": 1, \"messages\": {messages},");
    std::fs::write(&config, text.replacen("\"version\": 1,", &block, 1)).unwrap();
}

fn without(issues: &Value, field: &str) -> Vec<Value> {
    issues
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| {
            let mut issue = issue.clone();
            issue.as_object_mut().unwrap().remove(field);
            issue
        })
        .collect()
}

#[test]
fn messages_change_gitlab_descriptions_and_no_fingerprint() {
    for fixture in ["tiny-ts-violations", "exemplar-stratum-app"] {
        let repo = repo_with_committed_fixture_in_subdir(fixture);
        let root = repo.path().join(PROJECT_SUBDIR);
        let plain = json(&root, &["--format", "gitlab", "--no-baseline"]);
        with_messages(&root, &messages());
        let overridden = json(&root, &["--format", "gitlab", "--no-baseline"]);
        assert!(!plain.as_array().unwrap().is_empty(), "{fixture}");
        assert_eq!(
            without(&plain, "description"),
            without(&overridden, "description"),
            "{fixture}"
        );
        for (plain, overridden) in plain
            .as_array()
            .unwrap()
            .iter()
            .zip(overridden.as_array().unwrap())
        {
            let rule = overridden["check_name"].as_str().unwrap();
            let description = overridden["description"].as_str().unwrap();
            assert!(
                description.starts_with(&format!("{}. {MARK}", title(rule))),
                "{description}"
            );
            assert!(!plain["description"].as_str().unwrap().contains(MARK));
        }
    }
}

#[test]
fn messages_keep_the_baseline_written_without_them() {
    let repo = repo_with_committed_fixture_in_subdir("tiny-ts-violations");
    let root = repo.path().join(PROJECT_SUBDIR);
    assert_eq!(stratum(&root, &["baseline"]).status.code(), Some(0));
    let plain_baseline = std::fs::read_to_string(root.join("stratum.baseline.json")).unwrap();

    with_messages(&root, &messages());
    let out_path = root.join("overridden.baseline.json");
    let out = stratum(&root, &["baseline", "--out", out_path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(std::fs::read_to_string(&out_path).unwrap(), plain_baseline);

    let all = json(&root, &["--format", "json", "--no-baseline"]);
    let violations = all["violations"].as_array().unwrap();
    assert!(!violations.is_empty());
    assert!(
        violations
            .iter()
            .all(|v| v["message"].as_str().unwrap().contains(MARK))
    );

    let gated = json(&root, &["--format", "json"]);
    assert_eq!(gated["violations"], Value::Array(vec![]));
    assert_eq!(
        gated["summary"]["baselined"].as_u64(),
        Some(violations.len() as u64)
    );
    assert_eq!(gated["summary"]["fixed"].as_u64(), Some(0));
}

#[test]
fn a_bad_messages_block_is_a_config_error_naming_the_key() {
    let cases = [
        (
            serde_json::json!({ "texts": { "NoSuchKey": "x" } }),
            "NoSuchKey",
        ),
        (
            serde_json::json!({ "texts": { "UnusedExport": "`{nme}` is unused" } }),
            "messages.texts.UnusedExport",
        ),
        (
            serde_json::json!({ "titles": { "stratum/no-such-rule": "x" } }),
            "stratum/no-such-rule",
        ),
    ];
    for (block, named) in cases {
        let repo = repo_with_committed_fixture_in_subdir("tiny-ts-violations");
        let root = repo.path().join(PROJECT_SUBDIR);
        with_messages(&root, &block);
        let out = stratum(&root, &["--format", "json", "--no-baseline"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{named}: {stderr}");
        assert!(stderr.contains(named), "{named}: {stderr}");
    }
}

#[test]
fn messages_command_lists_every_key_and_rule() {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["messages", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["texts"].as_array().unwrap().len(), Key::ALL.len());
    assert_eq!(
        value["rules"].as_array().unwrap().len(),
        RuleRegistry::with_builtins().rules().len()
    );
}
