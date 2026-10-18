#![allow(clippy::unwrap_used)]

mod support;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use assert_cmd::Command;
use serde_json::Value;
use support::{copy_dir, fixture};

const SOURCE_EXTENSIONS: [&str; 9] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "vue"];

fn lint(root: &Path, cache: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::cargo_bin("stratum-lint")
        .unwrap()
        .current_dir(root)
        .args([".", "--no-baseline", "--cache-dir"])
        .arg(cache)
        .args(args)
        .output()
        .unwrap();
    let code = out.status.code().unwrap();
    assert!(
        code <= 1,
        "{}: tool failure\n{}",
        root.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    (code, String::from_utf8(out.stdout).unwrap())
}

fn rows(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn sources(dir: &Path, root: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "node_modules") {
                sources(&path, root, out);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| SOURCE_EXTENSIONS.contains(&e))
        {
            let relative = path.strip_prefix(root).unwrap();
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn case_dirs() -> Vec<PathBuf> {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/fixtures");
    let mut dirs = vec![fixture("exemplar-stratum-app")];
    dirs.extend(sorted_subdirs(&fixture(".")).into_iter().filter(|p| {
        p.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("tiny-"))
    }));
    for rule in sorted_subdirs(&fixtures) {
        for kind in ["bad", "good"] {
            dirs.extend(sorted_subdirs(&rule.join(kind)));
        }
    }
    dirs
}

fn sorted_subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|e| e.unwrap().path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs
}

fn cut_ends(row: &Value) -> impl Iterator<Item = &str> {
    row["fix"]
        .as_str()
        .unwrap_or_default()
        .split('`')
        .skip(1)
        .step_by(2)
        .flat_map(|cut| cut.split(" -> "))
        .map(|end| end.rsplit_once(':').map_or(end, |(path, _)| path))
}

fn touches(file: &str, row: &Value, violation: &Value) -> bool {
    let edge = &violation["edge"];
    row["file"] == file
        || edge["from"] == file
        || edge["to"] == file
        || (edge.is_null() && cut_ends(row).any(|end| end == file))
}

#[test]
fn files_scope_equals_the_full_run_filtered_to_that_file_for_every_module() {
    let cache = tempfile::tempdir().unwrap();
    let mut checked = 0;
    for (n, project) in case_dirs().iter().enumerate() {
        let cache = cache.path().join(n.to_string());
        let full = rows(&lint(project, &cache, &["--format", "jsonl"]).1);
        let json: Value =
            serde_json::from_str(&lint(project, &cache, &["--format", "json"]).1).unwrap();
        let violations = json["violations"].as_array().unwrap();
        assert_eq!(full.len(), violations.len(), "{}", project.display());

        let mut modules = Vec::new();
        sources(project, project, &mut modules);
        for file in modules {
            let expected: Vec<&Value> = full
                .iter()
                .zip(violations)
                .filter(|(row, violation)| touches(&file, row, violation))
                .map(|(row, _)| row)
                .collect();
            let got = rows(&lint(project, &cache, &["--format", "jsonl", "--files", &file]).1);
            assert_eq!(
                got.iter().collect::<Vec<_>>(),
                expected,
                "{}: --files {file}",
                project.display()
            );
            checked += 1;
        }
    }
    assert!(checked > 150, "only {checked} modules were compared");
}

fn age_files(dir: &Path) {
    let old = SystemTime::now() - Duration::from_secs(3600);
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            age_files(&path);
        } else {
            let file = std::fs::File::options().write(true).open(&path).unwrap();
            file.set_modified(old).unwrap();
        }
    }
}

/// The copy's files are an hour old, so the cache trusts them by size and mtime, not by hash.
/// `changes_findings`: the mutation must show in the output, else the comparison proves nothing.
fn warm_equals_cold_after(mutate: impl FnOnce(&Path, &Path), changes_findings: bool) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("app");
    copy_dir(&fixture("tiny-ts-violations"), &root);
    age_files(&root);
    let cache = dir.path().join("cache");
    let jsonl = ["--format", "jsonl"];

    let before = lint(&root, &cache, &jsonl);
    mutate(&root, &cache);
    let warm = lint(&root, &cache, &jsonl);
    let cold = lint(&root, &dir.path().join("fresh"), &jsonl);

    assert_eq!(warm, cold);
    if changes_findings {
        assert_ne!(warm, before, "the mutation did not change the findings");
    }
}

fn replace_in(path: &Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{} lacks {from}", path.display());
    std::fs::write(path, text.replace(from, to)).unwrap();
}

fn cache_file(cache: &Path) -> PathBuf {
    std::fs::read_dir(cache)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "graph"))
        .unwrap()
}

#[test]
fn a_warm_run_sees_an_edited_file() {
    warm_equals_cold_after(
        |root, _| {
            let http = root.join("src/shared/api/http.ts");
            std::fs::write(http, "export const http = 1;\n").unwrap();
        },
        true,
    );
}

#[test]
fn a_warm_run_sees_an_added_file() {
    warm_equals_cold_after(
        |root, _| {
            std::fs::write(
                root.join("src/shared/leak.ts"),
                "import { Cart } from '@/features/cart/cart';\nexport const leak = Cart;\n",
            )
            .unwrap();
        },
        true,
    );
}

#[test]
fn a_warm_run_sees_a_deleted_file() {
    warm_equals_cold_after(
        |root, _| std::fs::remove_file(root.join("src/features/cart/circle.ts")).unwrap(),
        true,
    );
}

#[test]
fn a_warm_run_sees_a_renamed_file_that_keeps_its_size_and_mtime() {
    warm_equals_cold_after(
        |root, _| {
            let cart = root.join("src/features/cart");
            std::fs::rename(cart.join("cart.tsx"), cart.join("trolley.tsx")).unwrap();
        },
        true,
    );
}

#[test]
fn a_warm_run_sees_a_config_edit() {
    warm_equals_cold_after(
        |root, _| {
            replace_in(
                &root.join("stratum.config.jsonc"),
                "\"stratum/purity\":          { \"severity\": \"error\" }",
                "\"stratum/purity\":          { \"severity\": \"off\" }",
            );
        },
        true,
    );
}

#[test]
fn a_warm_run_sees_a_tsconfig_paths_edit() {
    warm_equals_cold_after(
        |root, _| {
            replace_in(
                &root.join("tsconfig.json"),
                "\"@/*\": [\"src/*\"]",
                "\"@/*\": [\"lib/*\"]",
            );
        },
        true,
    );
}

#[test]
fn a_warm_run_survives_a_corrupt_cache_file() {
    warm_equals_cold_after(
        |_, cache| {
            let file = cache_file(cache);
            let mut bytes = std::fs::read(&file).unwrap();
            let middle = bytes.len() / 2;
            bytes[middle] ^= 0xff;
            std::fs::write(file, bytes).unwrap();
        },
        false,
    );
}

#[test]
fn a_warm_run_survives_a_truncated_cache_file() {
    warm_equals_cold_after(
        |_, cache| {
            let file = cache_file(cache);
            let bytes = std::fs::read(&file).unwrap();
            std::fs::write(file, &bytes[..bytes.len() / 2]).unwrap();
        },
        false,
    );
}
