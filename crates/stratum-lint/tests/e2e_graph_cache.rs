#![allow(clippy::unwrap_used)]

mod support;

use std::path::Path;

use assert_cmd::Command;
use support::{copy_dir, fixture};

fn lint(root: &Path, cache: &Path, format: &str) -> String {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(root)
        .args(["--no-baseline", "--format", format, "--cache-dir"])
        .arg(cache)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn without_run_fields(json: &str) -> serde_json::Value {
    let mut report: serde_json::Value = serde_json::from_str(json).unwrap();
    let summary = report["summary"].as_object_mut().unwrap();
    summary.remove("elapsed_ms");
    summary.remove("cache");
    report
}

fn without_elapsed(text: &str) -> String {
    let (head, rest) = text.split_once('\n').unwrap_or((text, ""));
    let head = match head.rsplit_once(", ") {
        Some((counts, last))
            if last.ends_with('s') && last[..last.len() - 1].parse::<f64>().is_ok() =>
        {
            counts
        }
        _ => head,
    };
    format!("{head}\n{rest}")
}

#[test]
fn a_warm_run_prints_what_a_cold_run_prints() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("app");
    copy_dir(&fixture("tiny-ts-violations"), &root);
    std::fs::write(
        root.join("src/shared/leak.ts"),
        "// stratum-ignore stratum/no-cross-layer-import -- kept on purpose\nimport { cart } from '../features/cart/index';\nexport const leak = cart;\n",
    )
    .unwrap();

    for format in ["terminal", "jsonl", "agent", "sarif", "json"] {
        let cache = dir.path().join(format!("cache-{format}"));
        let cold = lint(&root, &cache, format);
        let warm = lint(&root, &cache, format);
        if format == "json" {
            assert_eq!(without_run_fields(&cold), without_run_fields(&warm));
            let state = |json: &str| {
                serde_json::from_str::<serde_json::Value>(json).unwrap()
                ["summary"]["cache"]["state"]
                .clone()
            };
            assert_eq!((state(&cold), state(&warm)), ("cold".into(), "warm".into()));
        } else {
            assert_eq!(without_elapsed(&cold), without_elapsed(&warm), "{format}");
        }
    }
}
