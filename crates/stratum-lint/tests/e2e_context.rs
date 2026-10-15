#![allow(clippy::unwrap_used)]

use assert_cmd::Command;

const FIXTURE: &str = "../../tests/fixtures/tiny-ts-violations";

fn stdout(args: &[&str]) -> String {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(args)
        .assert()
        .success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn context_of_existing_module() {
    let file = format!("{FIXTURE}/src/features/cart/index.ts");
    insta::assert_snapshot!(stdout(&["context", &file]));
}

#[test]
fn context_of_file_not_created_yet() {
    let file = format!("{FIXTURE}/src/shared/api/new-client.ts");
    insta::assert_snapshot!(stdout(&["context", &file]));
}

#[test]
fn explain_unknown_slug_exits_two_and_lists_valid_slugs() {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["explain", "no-such-rule", FIXTURE])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("stratum/no-cross-layer-import"));
}
