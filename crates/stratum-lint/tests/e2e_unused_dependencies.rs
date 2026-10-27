#![allow(clippy::unwrap_used)]

use std::path::Path;

use assert_cmd::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "package.json",
        r#"{
  "name": "app",
  "scripts": { "lint": "eslint . && tsc --noEmit" },
  "dependencies": {
    "solid-js": "1",
    "left-pad": "1",
    "@scope/pkg": "1"
  },
  "devDependencies": {
    "@types/lodash": "4",
    "@types/node": "20",
    "@types/jest": "29",
    "eslint": "9",
    "typescript": "5",
    "vitest": "2",
    "vite": "5"
  }
}
"#,
    );
    write(
        root,
        "node_modules/typescript/package.json",
        r#"{ "bin": { "tsc": "./bin/tsc", "tsserver": "./bin/tsserver" } }"#,
    );
    write(
        root,
        "src/app.ts",
        "import { render } from 'solid-js/web';\nimport type { Dictionary } from 'lodash';\nimport { readFile } from 'node:fs';\nimport '@scope/pkg/side-effect.css';\nimport chalk from 'chalk';\nexport const d: Dictionary<number> = {};\nrender; readFile; chalk;\n",
    );
    write(
        root,
        "src/app.test.ts",
        "import { it } from 'vitest';\nimport undeclared from 'test-only-undeclared';\nit; undeclared;\n",
    );
    write(
        root,
        "vite.config.ts",
        "import { defineConfig } from 'vite';\nexport default defineConfig({});\n",
    );
    dir
}

fn findings(root: &Path, options: &serde_json::Value) -> Vec<String> {
    let config = serde_json::json!({
        "version": 1,
        "project": { "ignore": ["**/*.test.ts"] },
        "layers": [{ "id": "app", "path": "src", "depends_on": [] }],
        "rules": { "stratum/unused-dependencies": { "severity": "error", "options": options } },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["--format", "json", "--no-baseline"])
        .arg(root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut found: Vec<String> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["rule"] == "stratum/unused-dependencies")
        .map(|v| {
            let message = v["message"].as_str().unwrap();
            let package = message.split('`').nth(1).unwrap();
            let kind = if message.contains("nothing imports") {
                "unused"
            } else {
                "unlisted"
            };
            format!(
                "{kind} {package} {}:{}",
                v["file"].as_str().unwrap(),
                v["location"]["line"]
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn reports_declared_packages_nothing_uses_and_imported_packages_not_declared() {
    let dir = project();
    assert_eq!(
        findings(dir.path(), &serde_json::json!({})),
        [
            "unlisted chalk src/app.ts:5",
            "unused @types/jest package.json:12",
            "unused left-pad package.json:6",
        ]
    );
}

#[test]
fn ignore_check_and_scripts_options_narrow_the_report() {
    let dir = project();
    assert_eq!(
        findings(
            dir.path(),
            &serde_json::json!({ "ignore": ["@types/*"], "check": ["unused"], "scripts": false }),
        ),
        [
            "unused eslint package.json:13",
            "unused left-pad package.json:6",
            "unused typescript package.json:14",
        ]
    );
}

#[test]
fn a_config_file_counts_only_while_entries_lists_it() {
    let dir = project();
    let found = findings(
        dir.path(),
        &serde_json::json!({ "entries": [], "check": ["unused"] }),
    );
    assert!(
        found.contains(&"unused vite package.json:16".to_owned()),
        "{found:?}"
    );
}

#[test]
fn a_finding_on_package_json_is_in_scope_only_when_package_json_changed() {
    let dir = project();
    findings(dir.path(), &serde_json::json!({}));
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(dir.path())
        .args(["--format", "json", "--no-baseline", "--files", "src/app.ts"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let files: Vec<&str> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["file"].as_str().unwrap())
        .collect();
    assert_eq!(files, ["src/app.ts"]);
}
