#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use camino::Utf8PathBuf;

fn fixture() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts-violations")
        .canonicalize_utf8()
        .unwrap()
}

fn post_tool_use(tool: &str, file: &str) -> String {
    let root = fixture();
    let event = serde_json::json!({
        "hook_event_name": "PostToolUse",
        "cwd": root,
        "tool_name": tool,
        "tool_input": { "file_path": root.join(file) },
    });
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("hook")
        .write_stdin(event.to_string())
        .assert()
        .success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn edit_of_violating_file_adds_context() {
    let stdout = post_tool_use("Edit", "src/features/cart/boom.ts");
    let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    let context = output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("stratum/no-circular-deps"), "{context}");
}

#[test]
fn edit_of_a_cycle_member_other_than_the_reported_file_shows_the_cycle() {
    let stdout = post_tool_use("Edit", "src/features/cart/circle.ts");
    assert!(stdout.contains("stratum/no-circular-deps"), "{stdout}");
}

#[test]
fn edit_of_a_jsx_file_is_checked() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let write = |rel: &str, text: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "stratum.config.jsonc",
        r#"{ "version": 1,
            "layers": [{ "id": "features", "path": "src/features", "depends_on": ["shared"] },
                       { "id": "shared", "path": "src/shared" }],
            "rules": { "stratum/no-cross-layer-import": { "severity": "error" } } }"#,
    );
    write("src/features/cart.js", "export const cart = 1;\n");
    write(
        "src/shared/button.jsx",
        "import { cart } from '../features/cart.js';\nexport const Button = () => cart;\n",
    );
    let event = serde_json::json!({
        "hook_event_name": "PostToolUse",
        "cwd": root,
        "tool_name": "Edit",
        "tool_input": { "file_path": root.join("src/shared/button.jsx") },
    });
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("hook")
        .write_stdin(event.to_string())
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("stratum/no-cross-layer-import"), "{stdout}");
}

#[test]
fn edit_of_file_with_only_info_findings_is_silent() {
    assert_eq!(post_tool_use("Write", "src/app/main.ts"), "");
}
