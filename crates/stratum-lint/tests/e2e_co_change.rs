#![allow(clippy::unwrap_used)]

mod support;

use std::path::Path;

use assert_cmd::Command;

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src/api")).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("src/api/schema.ts"), "export const v = 1;\n").unwrap();
    std::fs::write(root.join("src/app.ts"), "export const app = 1;\n").unwrap();
    std::fs::write(root.join("docs/api.md"), "# API\n").unwrap();
    let config = serde_json::json!({
        "version": 1,
        "layers": [{ "id": "app", "path": "src", "depends_on": [] }],
        "rules": {
            "stratum/depth-ratio": { "severity": "off" },
            "stratum/container-breadth": { "severity": "off" },
            "stratum/co-change": { "severity": "error", "options": [
                { "when": ["src/api/**"], "require": ["docs/**"], "message": "API changed without docs" }
            ] },
        },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    support::git(root, &["init", "-q"]);
    support::git(root, &["add", "."]);
    support::git(root, &["commit", "-q", "-m", "base"]);
    dir
}

fn edit(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    let mut source = std::fs::read_to_string(&path).unwrap();
    source.push_str(text);
    std::fs::write(path, source).unwrap();
}

fn lint(root: &Path, args: &[&str]) -> (Option<i32>, Vec<(String, String)>) {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(root)
        .args(["--format", "json", "--no-baseline"])
        .args(args)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let found = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["rule"].as_str().unwrap().to_owned(),
                v["message"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    (out.status.code(), found)
}

#[test]
fn a_when_file_changed_alone_fails_lint_changed_and_diff() {
    let dir = repo();
    edit(dir.path(), "src/api/schema.ts", "export const w = 2;\n");
    let (code, found) = lint(dir.path(), &["--changed"]);
    assert_eq!(code, Some(1));
    assert_eq!(
        found,
        [(
            "stratum/co-change".to_owned(),
            "API changed without docs: `src/api/schema.ts` changed, but no file matching `docs/**` did".to_owned()
        )]
    );

    let (code, found) = lint(dir.path(), &[]);
    assert_eq!((code, found.len()), (Some(0), 0), "inert outside a diff");

    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["diff", "--base", "HEAD"])
        .arg(dir.path())
        .output()
        .unwrap();
    let markdown = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{markdown}");
    assert!(
        markdown.contains("#### Files that must change together (1)")
            && markdown.contains("- API changed without docs: `src/api/schema.ts` changed"),
        "{markdown}"
    );
}

#[test]
fn changing_a_require_file_too_or_an_unrelated_file_passes() {
    let dir = repo();
    edit(dir.path(), "src/app.ts", "export const more = 2;\n");
    assert_eq!(lint(dir.path(), &["--changed"]), (Some(0), vec![]));
    edit(dir.path(), "src/api/schema.ts", "export const w = 2;\n");
    edit(dir.path(), "docs/api.md", "More.\n");
    assert_eq!(lint(dir.path(), &["--changed"]), (Some(0), vec![]));
}

#[test]
fn a_directive_in_the_changed_file_acknowledges_and_is_never_unused() {
    let dir = repo();
    edit(
        dir.path(),
        "src/api/schema.ts",
        "// stratum-ignore stratum/co-change -- internal rename, no API change\nexport const w = 2;\n",
    );
    assert_eq!(lint(dir.path(), &["--changed"]), (Some(0), vec![]));
    assert_eq!(lint(dir.path(), &[]), (Some(0), vec![]));
}
