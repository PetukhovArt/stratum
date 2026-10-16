#![allow(clippy::unwrap_used)]

mod support;

use assert_cmd::Command;
use support::{PROJECT_SUBDIR, repo_with_committed_fixture_in_subdir};

#[test]
fn an_unfetched_ref_is_a_tool_failure_that_hints_a_shallow_fetch() {
    let repo = repo_with_committed_fixture_in_subdir("tiny-ts-violations");

    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["--format", "json", "--no-baseline", "--changed=no-such-ref"])
        .arg(repo.path().join(PROJECT_SUBDIR))
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("git fetch --depth=1 origin no-such-ref"),
        "{stderr}"
    );
}
