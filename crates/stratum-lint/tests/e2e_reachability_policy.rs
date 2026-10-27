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
        "src/app/worker.ts",
        "import { helper } from '../shared/helper';\nimport type { Node } from '../shared/dom/types';\nhelper();\nexport type N = Node;\n",
    );
    write(
        root,
        "src/shared/helper.ts",
        "import { render } from './dom/render';\nexport const helper = () => render();\nexport async function later() { return import('./dom/lazy'); }\n",
    );
    write(
        root,
        "src/shared/dom/render.ts",
        "export const render = () => 1;\n",
    );
    write(root, "src/shared/dom/lazy.ts", "export const lazy = 1;\n");
    write(
        root,
        "src/shared/dom/types.ts",
        "export type Node = { id: number };\n",
    );
    write(root, "src/main.ts", "import './app/worker';\n");
    dir
}

fn findings(root: &Path, policy: &serde_json::Value) -> Vec<(String, u64, String)> {
    let config = serde_json::json!({
        "version": 1,
        "project": { "entry_points": ["src/main.ts"] },
        "layers": [
            { "id": "app", "path": "src/app", "depends_on": ["shared"] },
            { "id": "shared", "path": "src/shared", "depends_on": [] },
        ],
        "rules": {
            "stratum/depth-ratio": { "severity": "off" },
            "stratum/reachability-policy": { "severity": "error", "options": [policy] },
        },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(root)
        .args(["--format", "json", "--no-baseline"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["rule"] == "stratum/reachability-policy")
        .map(|v| {
            (
                v["file"].as_str().unwrap().to_owned(),
                v["location"]["line"].as_u64().unwrap(),
                v["message"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn a_denied_module_reached_through_a_chain_is_reported_with_the_chain() {
    let dir = project();
    let found = findings(
        dir.path(),
        &serde_json::json!({ "from": ["src/app/worker.ts"], "deny": ["src/shared/dom/**"], "message": "Workers have no DOM" }),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    let (file, line, message) = &found[0];
    assert_eq!((file.as_str(), *line), ("src/app/worker.ts", 2));
    assert_eq!(
        message,
        "Workers have no DOM: `src/app/worker.ts` reaches `src/shared/dom/types.ts` through src/app/worker.ts -> src/shared/dom/types.ts (and 2 more denied modules)"
    );
}

#[test]
fn ignored_edge_kinds_and_allow_narrow_what_is_reachable() {
    let dir = project();
    let found = findings(
        dir.path(),
        &serde_json::json!({
            "from": ["src/app/worker.ts"],
            "deny": ["src/shared/dom/**"],
            "allow": ["src/shared/dom/render.ts"],
            "ignore_type_only": true,
            "ignore_dynamic": true,
        }),
    );
    assert!(found.is_empty(), "{found:?}");
    let found = findings(
        dir.path(),
        &serde_json::json!({
            "from": ["src/app/worker.ts"],
            "deny": ["src/shared/dom/**"],
            "ignore_type_only": true,
            "ignore_dynamic": true,
        }),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].2.ends_with(
            "through src/app/worker.ts -> src/shared/helper.ts -> src/shared/dom/render.ts"
        ),
        "{found:?}"
    );
}

#[test]
fn an_entry_point_outside_the_layers_can_be_a_from() {
    let dir = project();
    let found = findings(
        dir.path(),
        &serde_json::json!({ "from": ["src/main.ts"], "deny": ["src/shared/dom/lazy.ts"] }),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "src/main.ts");
    assert!(
        found[0].2.contains(
            "src/main.ts -> src/app/worker.ts -> src/shared/helper.ts -> src/shared/dom/lazy.ts"
        ),
        "{found:?}"
    );
}

#[test]
fn a_policy_without_deny_is_a_config_error() {
    let dir = project();
    let config = serde_json::json!({
        "version": 1,
        "layers": [{ "id": "app", "path": "src/app", "depends_on": [] }],
        "rules": { "stratum/reachability-policy": { "severity": "error", "options": [{ "from": ["src/**"] }] } },
    });
    std::fs::write(dir.path().join("stratum.config.jsonc"), config.to_string()).unwrap();
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(dir.path())
        .arg("--no-baseline")
        .assert()
        .code(2);
}
