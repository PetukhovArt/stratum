#![allow(clippy::unwrap_used)]

use std::path::Path;

use assert_cmd::Command;

fn unused_names(root: &Path, options: &serde_json::Value) -> Vec<String> {
    let config = serde_json::json!({
        "version": 1,
        "project": { "entry_points": ["src/main.ts"], "ignore": ["**/*.test.ts"] },
        "layers": [{ "id": "app", "path": "src", "depends_on": [] }],
        "rules": { "stratum/unused-exports": { "severity": "error", "options": options } },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["--format", "json", "--no-baseline"])
        .arg(root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut names: Vec<String> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["rule"] == "stratum/unused-exports")
        .map(|v| {
            let message = v["message"].as_str().unwrap();
            message.split('`').nth(1).unwrap().to_owned()
        })
        .collect();
    names.sort();
    names
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("lib.ts"),
        "export const used = 1;\nexport const testOnly = 2;\nexport const dead = 3;\nexport type Props = { a: number };\nexport interface Shape { b: string }\n",
    )
    .unwrap();
    std::fs::write(
        src.join("main.ts"),
        "import { used } from './lib';\nused;\n",
    )
    .unwrap();
    std::fs::write(
        src.join("lib.test.ts"),
        "import { testOnly } from './lib';\ntestOnly;\n",
    )
    .unwrap();
    dir
}

#[test]
fn by_default_tests_do_not_count_and_types_are_reported() {
    let dir = project();
    assert_eq!(
        unused_names(dir.path(), &serde_json::json!({})),
        ["Props", "Shape", "dead", "testOnly"]
    );
}

#[test]
fn count_ignored_importers_takes_the_names_an_ignored_test_imports() {
    let dir = project();
    assert_eq!(
        unused_names(
            dir.path(),
            &serde_json::json!({ "count_ignored_importers": true })
        ),
        ["Props", "Shape", "dead"]
    );
}

#[test]
fn ignore_type_exports_skips_type_aliases_and_interfaces() {
    let dir = project();
    assert_eq!(
        unused_names(
            dir.path(),
            &serde_json::json!({ "count_ignored_importers": true, "ignore_type_exports": true })
        ),
        ["dead"]
    );
}

#[test]
fn an_ignored_importer_does_not_make_a_module_reachable() {
    let dir = project();
    let src = dir.path().join("src");
    std::fs::write(src.join("orphan.ts"), "export const orphan = 1;\n").unwrap();
    std::fs::write(
        src.join("orphan.test.ts"),
        "import { orphan } from './orphan';\norphan;\n",
    )
    .unwrap();
    let config = serde_json::json!({
        "version": 1,
        "project": { "entry_points": ["src/main.ts"], "ignore": ["**/*.test.ts"] },
        "layers": [{ "id": "app", "path": "src", "depends_on": [] }],
        "rules": {
            "stratum/unused-exports": { "severity": "error", "options": { "count_ignored_importers": true } },
            "stratum/unreachable-modules": { "severity": "error" },
        },
    });
    std::fs::write(dir.path().join("stratum.config.jsonc"), config.to_string()).unwrap();
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["--format", "json", "--no-baseline"])
        .arg(dir.path())
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let unreachable: Vec<&str> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["rule"] == "stratum/unreachable-modules")
        .map(|v| v["file"].as_str().unwrap())
        .collect();
    assert_eq!(unreachable, ["src/orphan.ts"]);
}
