#![allow(clippy::unwrap_used)]

mod support;

use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use support::{PROJECT_SUBDIR, copy_dir, fixture, git};

const BASELINE: &str = "stratum.baseline.json";

fn stratum(args: &[&str], root: &Path) -> Output {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(args)
        .arg(root)
        .output()
        .unwrap()
}

fn commit_all(repo: &Path) {
    git(repo, &["add", "."]);
    git(repo, &["commit", "-q", "-m", "commit"]);
}

fn repo_with_baseline_at_head(subdir: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(subdir);
    copy_dir(&fixture("tiny-ts-violations"), &root);
    git(dir.path(), &["init", "-q"]);
    assert_eq!(stratum(&["baseline"], &root).status.code(), Some(0));
    commit_all(dir.path());
    (dir, root)
}

/// Edits the version 2 entries `<rule> <file> <digest>[ x<count>][ <touch>...]` as token lists.
fn edit_violations(root: &Path, edit: impl FnOnce(&mut Vec<Vec<String>>)) {
    let path = root.join(BASELINE);
    let mut json: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut entries: Vec<Vec<String>> = json["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| {
            line.as_str()
                .unwrap()
                .split(' ')
                .map(str::to_owned)
                .collect()
        })
        .collect();
    edit(&mut entries);
    json["violations"] = entries.iter().map(|tokens| tokens.join(" ")).collect();
    std::fs::write(path, json.to_string()).unwrap();
}

fn check(root: &Path, reference: &str) -> (Option<i32>, String, String) {
    let out = stratum(&["baseline", &format!("--check={reference}")], root);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn moving_a_violation_to_another_file_at_the_same_total_grows() {
    let (_repo, root) = repo_with_baseline_at_head("");
    edit_violations(&root, |v| {
        v[1][1] = "src/features/cart/circle.ts".into();
    });

    let (code, stdout, _) = check(&root, "HEAD");

    assert_eq!(code, Some(1), "{stdout}");
    assert!(
        stdout.contains("+1 stratum/no-circular-deps src/features/cart/circle.ts "),
        "{stdout}"
    );
    assert!(!stdout.contains("src/features/cart/boom.ts"), "{stdout}");
}

#[test]
fn a_count_rising_from_one_to_two_grows() {
    let (_repo, root) = repo_with_baseline_at_head("");
    edit_violations(&root, |v| v[0].insert(3, "x2".into()));

    let (code, stdout, _) = check(&root, "HEAD");

    assert_eq!(code, Some(1), "{stdout}");
    assert!(
        stdout.contains("+1 stratum/depth-ratio src/shared/api/http.ts"),
        "{stdout}"
    );
}

#[test]
fn a_pruned_baseline_passes() {
    let (_repo, root) = repo_with_baseline_at_head("");
    edit_violations(&root, |v| {
        v.remove(1);
    });

    let (code, stdout, _) = check(&root, "HEAD");

    assert_eq!(code, Some(0), "{stdout}");
}

#[test]
fn a_baseline_absent_at_the_ref_is_introduced() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    copy_dir(&fixture("tiny-ts-violations"), &root);
    git(&root, &["init", "-q"]);
    commit_all(&root);
    assert_eq!(stratum(&["baseline"], &root).status.code(), Some(0));

    let (code, stdout, _) = check(&root, "HEAD");

    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.contains("introduced"), "{stdout}");
}

#[test]
fn no_baseline_anywhere_passes_but_an_unknown_ref_still_fails() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    copy_dir(&fixture("tiny-ts-violations"), &root);
    git(&root, &["init", "-q"]);
    commit_all(&root);

    let (code, stdout, stderr) = check(&root, "HEAD");

    assert_eq!(code, Some(0), "{stdout}{stderr}");
    assert!(stdout.contains("nothing to check"), "{stdout}");

    let (code, _, stderr) = check(&root, "no-such-ref");

    assert_eq!(code, Some(2), "{stderr}");
}

#[test]
fn an_unknown_ref_is_a_tool_failure_that_hints_a_shallow_fetch() {
    let (_repo, root) = repo_with_baseline_at_head("");

    let (code, _, stderr) = check(&root, "no-such-ref");

    assert_eq!(code, Some(2), "{stderr}");
    assert!(
        stderr.contains("git fetch --depth=1 origin no-such-ref"),
        "{stderr}"
    );
}

#[test]
fn root_in_a_repo_subdirectory_reads_its_own_baseline_at_the_ref() {
    let (_repo, root) = repo_with_baseline_at_head(PROJECT_SUBDIR);
    edit_violations(&root, |v| v[0].insert(3, "x2".into()));

    let (code, stdout, _) = check(&root, "HEAD");

    assert_eq!(code, Some(1), "{stdout}");
    assert!(
        stdout.contains("+1 stratum/depth-ratio src/shared/api/http.ts"),
        "{stdout}"
    );
}

/// A version 1 baseline as 0.4.0 wrote it: one object per violation, edge endpoints as touches.
fn v1_baseline_of_current_violations(root: &Path) -> String {
    let out = stratum(&["--format", "json", "--no-baseline"], root);
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let violations: Vec<Value> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let mut entry = serde_json::json!({
                "rule": v["rule"], "file": v["file"], "message": v["message"],
            });
            let touches: Vec<Value> = ["from", "to"]
                .iter()
                .map(|end| v["edge"][end].clone())
                .filter(|t| t.is_string() && *t != v["file"])
                .collect();
            if !touches.is_empty() {
                entry["touches"] = touches.into();
            }
            entry
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({ "version": 1, "violations": violations }))
        .unwrap()
}

#[test]
fn a_v1_baseline_still_gates_and_the_next_write_converts_it_to_v2() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    copy_dir(&fixture("tiny-ts-violations"), &root);
    git(&root, &["init", "-q"]);
    std::fs::write(
        root.join(BASELINE),
        v1_baseline_of_current_violations(&root),
    )
    .unwrap();
    commit_all(&root);

    let lint = stratum(&["--format", "json"], &root);
    let report: Value = serde_json::from_slice(&lint.stdout).unwrap();
    assert_eq!(lint.status.code(), Some(0), "{report}");
    assert_eq!(report["summary"]["baselined"], 6, "{report}");
    assert_eq!(report["violations"].as_array().unwrap().len(), 0);

    assert_eq!(
        stratum(&["baseline", "--prune"], &root).status.code(),
        Some(0)
    );
    let text = std::fs::read_to_string(root.join(BASELINE)).unwrap();
    assert!(
        text.starts_with("{\"version\":2,\"violations\":[\n\"depth-ratio "),
        "{text}"
    );
    assert_eq!(text.lines().count(), 2 + 6, "{text}");

    let (code, stdout, _) = check(&root, "HEAD");
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.contains("did not grow"), "{stdout}");
}
