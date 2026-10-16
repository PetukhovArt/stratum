#![allow(clippy::unwrap_used)]

mod support;

use std::path::Path;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use support::{PROJECT_SUBDIR, git, repo_with_committed_fixture_in_subdir};

fn diff(root: &Path) -> (i32, String) {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .args(["diff", "--base", "HEAD"])
        .arg(root)
        .output()
        .unwrap();
    let sha = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap()
        .stdout;
    let sha = String::from_utf8(sha).unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    (
        out.status.code().unwrap(),
        stdout.replace(sha.trim(), "<sha>"),
    )
}

fn append(path: &Path, text: &str) {
    let old = std::fs::read_to_string(path).unwrap();
    std::fs::write(path, format!("{old}{text}")).unwrap();
}

#[test]
fn drift_report_lists_a_new_slice_edge_and_exits_1() {
    let repo = repo_with_committed_fixture_in_subdir("exemplar-stratum-app");
    let root = repo.path().join(PROJECT_SUBDIR);
    append(
        &root.join("src/features/checkout/index.ts"),
        "import \"@/features/cart\";\n",
    );

    let (code, stdout) = diff(&root);

    assert_eq!(code, 1, "{stdout}");
    insta::assert_snapshot!(stdout);
}

#[test]
fn unchanged_tree_reports_no_drift_and_exits_0() {
    let repo = repo_with_committed_fixture_in_subdir("exemplar-stratum-app");

    let (code, stdout) = diff(&repo.path().join(PROJECT_SUBDIR));

    assert_eq!(code, 0, "{stdout}");
    insta::assert_snapshot!(stdout);
}

#[test]
fn report_stays_within_8_kib_and_ends_with_the_local_command() {
    let repo = repo_with_committed_fixture_in_subdir("exemplar-stratum-app");
    let shared = repo.path().join(PROJECT_SUBDIR).join("src/shared");
    for cycle in 0..12 {
        for i in 0..5 {
            let name = |i: usize| format!("cycle{cycle}-member{i}-{}", "x".repeat(150));
            std::fs::write(
                shared.join(format!("{}.ts", name(i))),
                format!("import \"./{}\";\n", name((i + 1) % 5)),
            )
            .unwrap();
        }
    }

    let (code, stdout) = diff(&repo.path().join(PROJECT_SUBDIR));

    assert_eq!(code, 1);
    assert!(stdout.len() <= 8 * 1024, "{} bytes", stdout.len());
    assert!(
        stdout
            .trim_end()
            .ends_with("run `stratum-lint diff --base HEAD` locally for the full report."),
        "{stdout}"
    );
}

#[test]
fn base_tsconfig_extending_a_package_keeps_its_aliases() {
    let repo = repo_with_committed_fixture_in_subdir("exemplar-stratum-app");
    let root = repo.path().join(PROJECT_SUBDIR);
    let package = root.join("node_modules/@acme/tsconfig");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("base.json"),
        r#"{ "compilerOptions": { "paths": { "@/*": ["src/*"] } } }"#,
    )
    .unwrap();
    std::fs::write(
        root.join("tsconfig.json"),
        r#"{ "extends": "@acme/tsconfig/base.json", "compilerOptions": { "baseUrl": "." } }"#,
    )
    .unwrap();
    std::fs::write(root.join(".gitignore"), "node_modules\n").unwrap();
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-q", "-m", "extends a package"]);

    let (code, stdout) = diff(&root);

    assert_eq!(code, 0, "{stdout}");
}
