#![allow(clippy::unwrap_used)]

mod support;

use std::collections::BTreeSet;

use assert_cmd::Command;
use support::{PROJECT_SUBDIR, repo_with_committed_fixture_in_subdir};

#[test]
fn changed_scope_covers_untracked_and_modified_files_when_root_is_a_repo_subdir() {
    let repo = repo_with_committed_fixture_in_subdir("tiny-ts-violations");
    let root = repo.path().join(PROJECT_SUBDIR);
    std::fs::write(
        root.join("src/shared/leak.ts"),
        "import { cart } from '../features/cart/index';\nexport const leak = cart;\n",
    )
    .unwrap();
    let http = root.join("src/shared/api/http.ts");
    let mut source = std::fs::read_to_string(&http).unwrap();
    source.push('\n');
    std::fs::write(&http, source).unwrap();

    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["--format", "json", "--no-baseline", "--changed"])
        .arg(&root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let files: BTreeSet<&str> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["file"].as_str().unwrap())
        .collect();

    assert!(files.contains("src/shared/leak.ts"), "untracked: {files:?}");
    assert!(
        files.contains("src/shared/api/http.ts"),
        "modified: {files:?}"
    );
    assert!(
        !files.contains("src/features/cart/boom.ts"),
        "unchanged: {files:?}"
    );
}
