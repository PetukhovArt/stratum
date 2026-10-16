#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/fixtures")
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|e| e.unwrap().path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs
}

fn findings(case: &Path) -> Vec<Value> {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(case)
        .args(["--format", "jsonl", "--no-baseline"])
        .output()
        .unwrap();
    assert!(
        out.status.code().is_some_and(|c| c <= 1),
        "{}: tool failure\n{}",
        case.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn expected(case: &Path) -> Vec<Value> {
    let text = fs::read_to_string(case.join("expected.jsonl")).unwrap_or_default();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn is_found(want: &Value, got: &[Value]) -> bool {
    let want = want.as_object().unwrap();
    got.iter()
        .any(|g| want.iter().all(|(key, value)| g.get(key) == Some(value)))
}

#[test]
fn bad_cases_report_every_expected_finding_and_good_cases_nothing() {
    let mut failures = Vec::new();
    let mut cases = 0;
    for rule in subdirs(&fixtures()) {
        for case in subdirs(&rule.join("bad")) {
            cases += 1;
            let want = expected(&case);
            let got = findings(&case);
            if want.is_empty() {
                failures.push(format!("{}: expected.jsonl is empty", case.display()));
            }
            for w in want.iter().filter(|w| !is_found(w, &got)) {
                failures.push(format!("{}: missing {w}", case.display()));
            }
        }
        for case in subdirs(&rule.join("good")) {
            cases += 1;
            for g in findings(&case) {
                failures.push(format!("{}: unexpected {g}", case.display()));
            }
        }
    }
    assert!(cases > 0, "no eval cases under {}", fixtures().display());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
