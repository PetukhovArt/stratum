#![allow(clippy::unwrap_used)]

mod support;

use std::path::Path;

use assert_cmd::Command;
use camino::Utf8PathBuf;
use support::{PROJECT_SUBDIR, repo_with_committed_fixture_in_subdir};

fn fixture() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts-violations")
        .canonicalize_utf8()
        .unwrap()
}

fn post_tool_use(tool: &str, file: &str) -> String {
    let root = fixture();
    let event = serde_json::json!({
        "hook_event_name": "PostToolUse",
        "cwd": root,
        "tool_name": tool,
        "tool_input": { "file_path": root.join(file) },
    });
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("hook")
        .write_stdin(event.to_string())
        .assert()
        .success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn edit_of_violating_file_adds_context() {
    let stdout = post_tool_use("Edit", "src/features/cart/boom.ts");
    let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    let context = output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("stratum/no-circular-deps"), "{context}");
}

#[test]
fn edit_of_a_cycle_member_other_than_the_reported_file_shows_the_cycle() {
    let stdout = post_tool_use("Edit", "src/features/cart/circle.ts");
    assert!(stdout.contains("stratum/no-circular-deps"), "{stdout}");
}

#[test]
fn edit_of_file_with_only_info_findings_is_silent() {
    assert_eq!(post_tool_use("Write", "src/app/main.ts"), "");
}

fn stop(cwd: &Path, stop_hook_active: bool) -> assert_cmd::assert::Assert {
    let event = serde_json::json!({
        "hook_event_name": "Stop",
        "cwd": cwd,
        "stop_hook_active": stop_hook_active,
    });
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("hook")
        .env(
            "GIT_CEILING_DIRECTORIES",
            cwd.parent().unwrap().parent().unwrap(),
        )
        .write_stdin(event.to_string())
        .assert()
}

fn repo_with_untracked_violation() -> tempfile::TempDir {
    let repo = repo_with_committed_fixture_in_subdir("tiny-ts-violations");
    std::fs::write(
        repo.path().join(PROJECT_SUBDIR).join("src/shared/leak.ts"),
        "import { cart } from '../features/cart/index';\nexport const leak = cart;\n",
    )
    .unwrap();
    repo
}

#[test]
fn stop_blocks_on_a_violation_in_an_untracked_file() {
    let repo = repo_with_untracked_violation();
    let out = stop(&repo.path().join(PROJECT_SUBDIR), false).success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let decision: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(decision["decision"], "block");
}

#[test]
fn stop_already_continued_by_the_hook_never_blocks_again() {
    let repo = repo_with_untracked_violation();
    stop(&repo.path().join(PROJECT_SUBDIR), true)
        .code(0)
        .stdout("");
}

#[test]
fn stop_outside_a_git_repo_is_a_non_blocking_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(PROJECT_SUBDIR);
    support::copy_dir(&support::fixture("tiny-ts-violations"), &root);
    stop(&root, false).code(1).stdout("");
}
