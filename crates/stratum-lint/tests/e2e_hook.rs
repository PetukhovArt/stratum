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
fn edit_of_file_with_only_info_findings_is_silent() {
    assert_eq!(post_tool_use("Write", "src/app/main.ts"), "");
}
