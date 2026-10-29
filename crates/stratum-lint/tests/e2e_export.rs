#![allow(clippy::unwrap_used)]

mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use assert_cmd::Command;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use camino::Utf8Path;
use http_body_util::BodyExt;
use serde_json::Value;
use stratum_lint::commands::{
    lint,
    visualize::{Api, build_router},
};
use support::{PROJECT_SUBDIR, repo_with_committed_fixture_in_subdir};
use tower::ServiceExt;

const SYSTEMS: &str = "../../tests/fixtures/tiny-systems/web";
const TINY: &str = "../../tests/fixtures/tiny-ts";

fn export(root: &Path, extra: &[&str]) -> tempfile::TempDir {
    let out = tempfile::tempdir().unwrap();
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("export")
        .arg(root)
        .arg("--out")
        .arg(out.path())
        .args(extra)
        .assert()
        .success();
    out
}

fn router_at(root: &Path, base: Option<&str>) -> Router {
    let root = Utf8Path::from_path(root).unwrap();
    let input = lint::build(root, &lint::load_config(root, None).unwrap()).unwrap();
    build_router(Arc::new(Api::new(input, base, &[]).unwrap()))
}

async fn get(router: &Router, uri: &str) -> (StatusCode, String) {
    let response = router
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

fn read(out: &Path, rel: &str) -> String {
    std::fs::read_to_string(out.join(rel)).unwrap()
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// `data/` and the page: the files that carry what the export learned about the project.
fn written_text(out: &Path) -> Vec<(PathBuf, String)> {
    let mut files = files_under(&out.join("data"));
    files.push(out.join("index.html"));
    files
        .into_iter()
        .map(|f| {
            let text = std::fs::read_to_string(&f).unwrap();
            (f, text)
        })
        .collect()
}

#[tokio::test]
async fn export_writes_the_same_answers_the_server_gives() {
    for root in [SYSTEMS, TINY] {
        let root = Path::new(root);
        let out = export(root, &[]);
        let out = out.path();
        let router = router_at(root, None);
        for (route, file) in [
            ("/api/snapshot", "data/snapshot.json"),
            ("/api/violations", "data/violations.json"),
            ("/api/features", "data/features.json"),
        ] {
            let (status, body) = get(&router, route).await;
            assert_eq!(status, StatusCode::OK, "{route}");
            assert_eq!(read(out, file), body, "{root:?} {file}");
        }
        let (status, systems) = get(&router, "/api/systems").await;
        match status {
            StatusCode::OK => assert_eq!(read(out, "data/systems.json"), systems),
            _ => assert!(!out.join("data/systems.json").exists(), "{root:?}"),
        }

        let edges: Value = serde_json::from_str(&read(out, "data/edges.json")).unwrap();
        let edges = edges.as_object().unwrap();
        let snapshot: Value = serde_json::from_str(&read(out, "data/snapshot.json")).unwrap();
        let pairs: Vec<(u64, u64)> = snapshot["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| (e["from"].as_u64().unwrap(), e["to"].as_u64().unwrap()))
            .collect();
        assert!(!pairs.is_empty(), "{root:?}");
        for (from, to) in pairs {
            let (_, server) = get(&router, &format!("/api/edge?from={from}&to={to}")).await;
            let server: Value = serde_json::from_str(&server).unwrap();
            assert_eq!(
                edges[&from.to_string()][&to.to_string()],
                server,
                "{root:?} edge {from}->{to}"
            );
        }

        for module in snapshot["modules"].as_array().unwrap() {
            let path = module["path"].as_str().unwrap();
            let (_, brief) = get(&router, &format!("/api/context?path={path}")).await;
            let id = module["id"].as_u64().unwrap();
            assert_eq!(
                read(out, &format!("data/context/{id}.txt")),
                brief,
                "{path}"
            );
        }
    }
}

#[test]
fn without_with_source_no_line_of_any_source_file_leaves_the_repo() {
    let out = export(Path::new(SYSTEMS), &[]);
    let out = out.path();
    assert!(!out.join("data/source").exists());
    assert!(read(out, "index.html").contains(r#"window.__STRATUM_STATIC__ = {"source":false"#));
    let systems = read(out, "data/systems.json");
    let fixture = Path::new(SYSTEMS).parent().unwrap();
    let source_lines: Vec<String> = files_under(fixture)
        .iter()
        .filter(|f| !f.ends_with("stratum.config.jsonc") && !f.ends_with("package.json"))
        .flat_map(|f| {
            std::fs::read_to_string(f)
                .unwrap()
                .lines()
                .map(str::trim)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|line| line.len() >= 12 && !systems.contains(line.as_str()))
        .collect();
    assert!(source_lines.len() > 20, "{source_lines:?}");
    for (file, text) in written_text(out) {
        for line in &source_lines {
            assert!(
                !text.contains(line.as_str()),
                "{file:?} holds the source line {line:?}"
            );
        }
    }
}

#[tokio::test]
async fn with_source_the_code_panel_files_are_written_as_the_server_reads_them() {
    let root = Path::new(SYSTEMS);
    let out = export(root, &["--with-source"]);
    let out = out.path();
    assert!(read(out, "index.html").contains(r#"{"source":true,"systems":true,"drift":false}"#));
    let index: Value = serde_json::from_str(&read(out, "data/source/index.json")).unwrap();
    let index = index.as_object().unwrap();
    assert!(index.contains_key("src/shared/api/users.ts"));
    assert!(index.contains_key("api:user.proto"), "{index:?}");
    assert!(index.contains_key("host:commands.rs"), "{index:?}");
    let router = router_at(root, None);
    for (path, n) in index {
        let file: Value =
            serde_json::from_str(&read(out, &format!("data/source/{n}.json"))).unwrap();
        let (status, server) = get(
            &router,
            &format!("/api/source?path={path}&line=1&context=200"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{path}");
        let server: Value = serde_json::from_str(&server).unwrap();
        assert_eq!(file["path"], server["path"], "{path}");
        assert_eq!(file["lines"], server["lines"], "{path}");
    }
}

#[tokio::test]
async fn with_base_the_drift_is_written_and_no_output_names_a_machine_path() {
    let repo = repo_with_committed_fixture_in_subdir("tiny-systems/web");
    let root = repo.path().join(PROJECT_SUBDIR);
    let out = export(&root, &["--with-source", "--base", "HEAD"]);
    let out = out.path();
    assert!(read(out, "index.html").contains(r#""drift":true"#));
    let (status, drift) = get(&router_at(&root, Some("HEAD")), "/api/drift").await;
    assert_eq!(status, StatusCode::OK);
    let drift: Value = serde_json::from_str(&drift).unwrap();
    let written: Value = serde_json::from_str(&read(out, "data/drift.json")).unwrap();
    assert_eq!(written["base"], drift["base"]);
    assert_eq!(written["edges"], drift["edges"]);

    let machine = repo.path().canonicalize().unwrap();
    let machine = machine.to_string_lossy();
    let machine = machine.trim_start_matches(r"\\?\");
    let forms = [machine.to_owned(), machine.replace('\\', "/")];
    let tmp = std::env::temp_dir();
    let tmp = tmp.to_string_lossy();
    let tmp_forms = [
        tmp.trim_end_matches('\\').to_owned(),
        tmp.replace('\\', "/").trim_end_matches('/').to_owned(),
    ];
    for (file, text) in written_text(out) {
        for form in forms.iter().chain(&tmp_forms) {
            assert!(!text.contains(form.as_str()), "{file:?} names {form}");
        }
    }
}

fn export_into(out: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("stratum-lint")
        .unwrap()
        .arg("export")
        .arg(TINY)
        .arg("--out")
        .arg(out)
        .assert()
}

#[test]
fn export_refuses_a_non_empty_folder_of_no_earlier_export_and_leaves_it_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path();
    std::fs::create_dir(out.join("data")).unwrap();
    std::fs::write(out.join("data/notes.txt"), "mine").unwrap();
    std::fs::write(out.join("index.html"), "mine").unwrap();
    let before = written_text(out);
    let stderr = export_into(out).code(2).get_output().stderr.clone();
    let stderr = String::from_utf8(stderr).unwrap();
    assert!(
        stderr.contains(out.file_name().unwrap().to_str().unwrap())
            && stderr.contains("earlier export"),
        "{stderr}"
    );
    assert_eq!(written_text(out), before);
    assert_eq!(files_under(out).len(), 2);
}

#[test]
fn export_creates_a_missing_folder_and_replaces_the_data_of_an_earlier_export() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("site");
    export_into(&out).success();
    std::fs::write(out.join("data/stale.json"), "{}").unwrap();
    export_into(&out).success();
    assert!(!out.join("data/stale.json").exists());
    assert!(out.join("data/snapshot.json").exists());
}
