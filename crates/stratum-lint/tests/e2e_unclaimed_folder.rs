#![allow(clippy::unwrap_used)]

use std::path::Path;

use assert_cmd::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn project(layers: &serde_json::Value) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for rel in [
        "src/features/editor/index.ts",
        "src/features/editor/core/model.ts",
        "src/features/editor/panels/frame.ts",
        "src/features/editor/panels/props/form.ts",
        "src/features/editor/_shared/scope.ts",
        "src/features/editor/ui/button.ts",
        "src/features/editor/legacy-paint/paint.ts",
        "src/features/editor/image-picker/picker.ts",
        "src/features/editor/image-picker/thumb.ts",
        "src/shared/http.ts",
    ] {
        write(root, rel, "export const x = 1;\n");
    }
    let config = serde_json::json!({
        "version": 1,
        "structure": { "private": ["_shared"], "segments": ["ui"] },
        "layers": [
            { "id": "editor-root", "path": "src/features/editor", "depends_on": ["core", "props", "shared"] },
            { "id": "core", "path": "src/features/editor/core" },
            { "id": "props", "path": "src/features/editor/panels/props" },
            { "id": "shared", "path": "src/shared" },
        ],
        "rules": {
            "stratum/unclaimed-folder": { "severity": "error", "options": {
                "layers": layers,
                "allow": ["src/features/editor/legacy-*"],
            } },
        },
    });
    std::fs::write(root.join("stratum.config.jsonc"), config.to_string()).unwrap();
    dir
}

fn lint(root: &Path, args: &[&str]) -> (Option<i32>, Vec<(String, u64, String)>) {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(root)
        .args(["--format", "json", "--no-baseline"])
        .args(args)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let found = report["violations"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|v| v["rule"] == "stratum/unclaimed-folder")
        .map(|v| {
            (
                v["file"].as_str().unwrap().to_owned(),
                v["location"]["line"].as_u64().unwrap(),
                v["message"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    (out.status.code(), found)
}

#[test]
fn only_the_folder_no_layer_private_zone_segment_or_allow_claims_is_reported() {
    let dir = project(&serde_json::json!(["editor-root"]));
    let (code, found) = lint(dir.path(), &[]);
    assert_eq!(code, Some(1));
    assert_eq!(
        found,
        [(
            "src/features/editor/image-picker/picker.ts".to_owned(),
            1,
            "`src/features/editor/image-picker` belongs to no layer of its own and falls into `editor-root`, which may import 3 layers".to_owned(),
        )]
    );
}

#[test]
fn files_scoping_reports_the_folder_through_any_of_its_modules() {
    let dir = project(&serde_json::json!(["editor-root"]));
    let (_, found) = lint(
        dir.path(),
        &["--files", "src/features/editor/image-picker/thumb.ts"],
    );
    assert_eq!(found.len(), 1);
    let (_, found) = lint(dir.path(), &["--files", "src/features/editor/index.ts"]);
    assert_eq!(found, []);
}

#[test]
fn an_unlisted_layer_is_not_checked_and_an_unknown_layer_id_is_a_config_error() {
    let dir = project(&serde_json::json!(["core"]));
    assert_eq!(lint(dir.path(), &[]), (Some(0), vec![]));
    let dir = project(&serde_json::json!(["editor"]));
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg(dir.path())
        .arg("--no-baseline")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("stratum/unclaimed-folder") && stderr.contains("\"editor\""),
        "{stderr}"
    );
}
