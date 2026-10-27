#![allow(clippy::unwrap_used)]

mod support;

use std::path::Path;

use assert_cmd::Command;

fn project(root: &Path) {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/a.ts"),
        "// @ts-ignore\nexport const a: number = 'x';\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/b.ts"),
        "/* eslint-disable no-console */\nexport const b = 1;\n",
    )
    .unwrap();
    let config = serde_json::json!({
        "version": 1,
        "layers": [{ "id": "app", "path": "src", "depends_on": [] }],
        "rules": {
            "stratum/depth-ratio": { "severity": "off" },
            "stratum/suppression": { "severity": "error", "options": { "forbid": ["eslint-disable", "@ts-ignore"] } },
        },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
}

fn forbidden(root: &Path, args: &[&str]) -> (Option<i32>, Vec<String>) {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut found: Vec<String> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["rule"] == "stratum/suppression")
        .map(|v| format!("{}:{}", v["file"].as_str().unwrap(), v["location"]["line"]))
        .collect();
    found.sort();
    (out.status.code(), found)
}

#[test]
fn each_forbidden_directive_is_an_error_on_its_line() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    assert_eq!(
        forbidden(dir.path(), &["--no-baseline"]),
        (
            Some(1),
            vec!["src/a.ts:1".to_owned(), "src/b.ts:1".to_owned()]
        )
    );
}

#[test]
fn with_a_baseline_only_a_new_directive_fails() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("baseline")
        .arg(dir.path())
        .assert()
        .success();
    assert_eq!(forbidden(dir.path(), &[]), (Some(0), vec![]));

    let a = dir.path().join("src/a.ts");
    let mut text = std::fs::read_to_string(&a).unwrap();
    text.push_str("// @ts-ignore\nexport const c: number = 'y';\n");
    std::fs::write(&a, text).unwrap();
    let (code, found) = forbidden(dir.path(), &[]);
    assert_eq!(code, Some(1));
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn changed_reports_only_directives_in_changed_files() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    support::git(dir.path(), &["init", "-q"]);
    support::git(dir.path(), &["add", "."]);
    support::git(dir.path(), &["commit", "-q", "-m", "base"]);
    let b = dir.path().join("src/b.ts");
    let mut text = std::fs::read_to_string(&b).unwrap();
    text.push_str("// eslint-disable-next-line\nexport const d = 2;\n");
    std::fs::write(&b, text).unwrap();
    assert_eq!(
        forbidden(dir.path(), &["--no-baseline", "--changed"]),
        (
            Some(1),
            vec!["src/b.ts:1".to_owned(), "src/b.ts:3".to_owned()]
        )
    );
}
