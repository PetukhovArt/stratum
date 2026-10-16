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

fn edit_violations(root: &Path, edit: impl FnOnce(&mut Vec<Value>)) {
    let path = root.join(BASELINE);
    let mut json: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    edit(json["violations"].as_array_mut().unwrap());
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
        v[1]["file"] = "src/features/cart/circle.ts".into();
    });

    let (code, stdout, _) = check(&root, "HEAD");

    assert_eq!(code, Some(1), "{stdout}");
    assert!(
        stdout.contains("+1 stratum/no-circular-deps src/features/cart/circle.ts Circular"),
        "{stdout}"
    );
    assert!(!stdout.contains("src/features/cart/boom.ts"), "{stdout}");
}

#[test]
fn a_count_rising_from_one_to_two_grows() {
    let (_repo, root) = repo_with_baseline_at_head("");
    edit_violations(&root, |v| v[0]["count"] = 2.into());

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
    edit_violations(&root, |v| v[0]["count"] = 2.into());

    let (code, stdout, _) = check(&root, "HEAD");

    assert_eq!(code, Some(1), "{stdout}");
    assert!(
        stdout.contains("+1 stratum/depth-ratio src/shared/api/http.ts"),
        "{stdout}"
    );
}
