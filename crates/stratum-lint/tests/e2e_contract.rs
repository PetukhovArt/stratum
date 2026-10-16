#![allow(clippy::unwrap_used)]

mod support;

use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use support::{copy_dir, fixture};

const HTTP: &str = "src/shared/api/http.ts";

fn violations_copy() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&fixture("tiny-ts-violations"), dir.path());
    dir
}

fn stratum(root: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn json(root: &Path, args: &[&str]) -> Value {
    let out = stratum(root, &[&["--format", "json"], args].concat());
    serde_json::from_slice(&out.stdout).unwrap()
}

fn edit(root: &Path, rel: &str, from: &str, to: &str) {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{rel} has no {from:?}");
    std::fs::write(path, text.replace(from, to)).unwrap();
}

#[test]
fn jsonl_line_has_exactly_the_contract_fields_in_order() {
    let root = fixture("tiny-ts-violations");
    let out = stratum(&root, &["--format", "jsonl", "--no-baseline"]);
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();

    let violations = json(&root, &["--no-baseline"])["violations"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(lines.len(), violations);

    let line = lines
        .iter()
        .find(|l| l.contains(r#""rule":"stratum/no-cross-layer-import""#))
        .unwrap();
    let expected_prefix = concat!(
        r#"{"v":1,"tool":"stratum","rule":"stratum/no-cross-layer-import","#,
        r#""severity":"warning","file":"src/shared/api/http.ts","line":2,"message":""#
    );
    assert!(line.starts_with(expected_prefix), "{line}");
    let parsed: Value = serde_json::from_str(line).unwrap();
    assert_eq!(parsed.as_object().unwrap().len(), 8);
    assert!(line.contains(r#","fix":""#));
    assert!(!parsed["fix"].as_str().unwrap().is_empty());
}

#[test]
fn a_clean_run_prints_nothing_in_jsonl() {
    let root = fixture("tiny-ts-violations");
    let out = stratum(
        &root,
        &[
            "--format",
            "jsonl",
            "--no-baseline",
            "--files",
            "src/app/main.ts",
        ],
    );
    assert!(out.stdout.is_empty(), "{}", stdout(&out));
    assert_eq!(out.status.code(), Some(0));
}

fn project_with_accepted_baseline() -> tempfile::TempDir {
    let dir = violations_copy();
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["baseline"])
        .arg(dir.path())
        .assert()
        .success();
    dir
}

fn drop_import_of_cart(root: &Path) {
    edit(
        root,
        HTTP,
        "import { Cart } from \"@/features/cart/cart\";\nexport const http = { Cart };",
        "export const http = {};",
    );
}

#[test]
fn exit_3_only_when_a_fixed_entry_is_in_scope() {
    let dir = project_with_accepted_baseline();
    drop_import_of_cart(dir.path());
    let baseline_args = ["--format", "agent"];

    let whole = stratum(dir.path(), &baseline_args);
    assert_eq!(whole.status.code(), Some(3), "{}", stdout(&whole));

    let scoped_to_fixed = stratum(
        dir.path(),
        &[&baseline_args[..], &["--files", HTTP]].concat(),
    );
    assert_eq!(scoped_to_fixed.status.code(), Some(3));

    let scoped_elsewhere = stratum(
        dir.path(),
        &[&baseline_args[..], &["--files", "src/app/main.ts"]].concat(),
    );
    assert_eq!(scoped_elsewhere.status.code(), Some(0));
    assert!(stdout(&scoped_elsewhere).contains("fixed"));
}

#[test]
fn exit_3_never_with_no_baseline_and_an_error_wins() {
    let dir = project_with_accepted_baseline();
    drop_import_of_cart(dir.path());
    let ignoring_baseline = stratum(dir.path(), &["--no-baseline", "--format", "agent"]);
    assert_eq!(ignoring_baseline.status.code(), Some(1));

    let cart = dir.path().join("src/features/cart");
    std::fs::write(cart.join("loop-a.ts"), "import \"./loop-b\";\n").unwrap();
    std::fs::write(cart.join("loop-b.ts"), "import \"./loop-a\";\n").unwrap();
    let new_error = stratum(dir.path(), &["--format", "agent"]);
    assert_eq!(new_error.status.code(), Some(1), "{}", stdout(&new_error));
}

const DIRECTIVE: &str =
    "// stratum-ignore stratum/no-cross-layer-import, stratum/public-api, stratum/purity";

fn with_directive(suffix: &str) -> tempfile::TempDir {
    let dir = violations_copy();
    edit(
        dir.path(),
        HTTP,
        "import { Cart }",
        &format!("{DIRECTIVE}{suffix}\nimport {{ Cart }}"),
    );
    dir
}

fn rules_on_http(report: &Value) -> Vec<&str> {
    report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["file"] == HTTP)
        .map(|v| v["rule"].as_str().unwrap())
        .collect()
}

#[test]
fn a_directive_without_a_reason_suppresses_nothing_and_is_an_error() {
    let dir = with_directive("");
    let report = json(dir.path(), &["--no-baseline"]);
    let rules = rules_on_http(&report);
    assert!(
        rules.contains(&"stratum/no-cross-layer-import"),
        "{rules:?}"
    );
    assert!(rules.contains(&"stratum/purity"));
    let problem = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["rule"] == "stratum/suppression")
        .unwrap();
    assert_eq!(problem["severity"], "error");
    assert_eq!(report["summary"]["suppressed"], 0);
}

#[test]
fn a_directive_with_a_reason_hides_its_line_everywhere_but_sarif() {
    let dir = with_directive(" -- generated bridge");
    let report = json(dir.path(), &["--no-baseline"]);
    assert_eq!(rules_on_http(&report), ["stratum/depth-ratio"]);
    assert_eq!(report["summary"]["suppressed"], 3);

    let jsonl = stdout(&stratum(
        dir.path(),
        &["--format", "jsonl", "--no-baseline"],
    ));
    assert!(!jsonl.contains("no-cross-layer-import"));

    let sarif: Value = serde_json::from_slice(
        &stratum(dir.path(), &["--format", "sarif", "--no-baseline"]).stdout,
    )
    .unwrap();
    let hidden: Vec<&Value> = sarif["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r.get("suppressions").is_some())
        .collect();
    assert_eq!(hidden.len(), 3);
    assert_eq!(hidden[0]["suppressions"][0]["kind"], "inSource");
    assert_eq!(
        hidden[0]["suppressions"][0]["justification"],
        "generated bridge"
    );

    let agent = stdout(&stratum(
        dir.path(),
        &["--format", "agent", "--no-baseline"],
    ));
    assert!(agent.lines().next().unwrap().contains(", 3 suppressed"));
}

#[test]
fn a_directive_that_hides_nothing_is_a_warning() {
    let dir = violations_copy();
    edit(
        dir.path(),
        "src/app/main.ts",
        "mountCart();",
        "// stratum-ignore stratum/purity -- stale\nmountCart();",
    );
    let report = json(dir.path(), &["--no-baseline"]);
    let unused = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["rule"] == "stratum/suppression")
        .unwrap();
    assert_eq!(unused["severity"], "warning");
    assert_eq!(unused["file"], "src/app/main.ts");
}
