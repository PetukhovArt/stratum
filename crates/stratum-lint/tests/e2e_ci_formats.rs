#![allow(clippy::unwrap_used)]

mod support;

use assert_cmd::Command;
use support::{PROJECT_SUBDIR, repo_with_committed_fixture_in_subdir};

fn lint(fixture: &str, format: &str) -> String {
    let repo = repo_with_committed_fixture_in_subdir(fixture);
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["--format", format, "--no-baseline"])
        .arg(repo.path().join(PROJECT_SUBDIR))
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn without_elapsed(text: &str) -> String {
    let seconds = |part: &str| {
        part.strip_suffix('s')
            .is_some_and(|n| n.parse::<f64>().is_ok())
    };
    text.lines()
        .map(|line| {
            if !line.starts_with("stratum: ") {
                return line.to_owned();
            }
            line.split(", ")
                .map(|part| if seconds(part) { "<elapsed>" } else { part })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn gitlab_tiny_ts_violations() {
    insta::assert_snapshot!(lint("tiny-ts-violations", "gitlab"));
}

#[test]
fn gitlab_exemplar() {
    insta::assert_snapshot!(lint("exemplar-stratum-app", "gitlab"));
}

#[test]
fn github_tiny_ts_violations() {
    insta::assert_snapshot!(without_elapsed(&lint("tiny-ts-violations", "github")));
}

#[test]
fn github_exemplar() {
    insta::assert_snapshot!(without_elapsed(&lint("exemplar-stratum-app", "github")));
}
