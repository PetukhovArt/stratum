#![allow(clippy::unwrap_used)]

mod support;

use std::sync::Arc;

use assert_cmd::Command;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use camino::Utf8Path;
use http_body_util::BodyExt;
use stratum_lint::commands::{
    lint,
    visualize::{Api, build_router},
};
use support::{PROJECT_SUBDIR, repo_with_committed_fixture_in_subdir};
use tower::ServiceExt;

const FIXTURE: &str = "../../tests/fixtures/tiny-ts-violations";

fn router_at(root: &Utf8Path, base: Option<&str>) -> Router {
    let input = lint::build(root, &lint::load_config(root, None).unwrap()).unwrap();
    build_router(Arc::new(Api::new(input, base).unwrap()))
}

async fn get(uri: &str) -> (StatusCode, String) {
    get_from(router_at(Utf8Path::new(FIXTURE), None), uri).await
}

async fn get_from(router: Router, uri: &str) -> (StatusCode, String) {
    let response = router
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn snapshot_route_returns_the_v3_snapshot() {
    let (status, body) = get("/api/snapshot").await;
    assert_eq!(status, StatusCode::OK);
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["version"], 3);
}

#[tokio::test]
async fn context_route_matches_the_context_command() {
    for rel in ["src/features/cart/index.ts", "src/shared/api/new-client.ts"] {
        let cli = Command::cargo_bin("stratum-lint")
            .unwrap()
            .args(["context", &format!("{FIXTURE}/{rel}")])
            .assert()
            .success();
        let cli = String::from_utf8(cli.get_output().stdout.clone()).unwrap();
        let (status, body) = get(&format!("/api/context?path={rel}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, cli, "{rel}");
    }
}

#[tokio::test]
async fn context_route_rejects_a_path_outside_the_root() {
    let (status, _) = get("/api/context?path=../../outside.ts").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn root_path_serves_embedded_index_html() {
    let (status, body) = get("/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Stratum Visualizer"));
}

#[tokio::test]
async fn unknown_asset_path_returns_404() {
    let (status, _) = get("/no-such-asset.js").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn drift_route_returns_the_drift_vs_base_with_its_markdown() {
    let repo = repo_with_committed_fixture_in_subdir("exemplar-stratum-app");
    let root = repo.path().join(PROJECT_SUBDIR);
    let file = root.join("src/features/checkout/index.ts");
    let old = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, format!("{old}import \"@/entities/user/model\";\n")).unwrap();

    let root = Utf8Path::from_path(&root).unwrap();
    let (status, body) = get_from(router_at(root, Some("HEAD")), "/api/drift").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let drift: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(drift["base"]["ref"], "HEAD");
    let markdown = drift["markdown"].as_str().unwrap();
    assert_eq!(
        markdown.lines().next(),
        Some(format!("<!-- stratum:drift {PROJECT_SUBDIR} -->").as_str())
    );
    assert!(markdown.contains(drift["base"]["sha"].as_str().unwrap()));
    let breaking: Vec<&serde_json::Value> = drift["breaking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| &drift["edges"][usize::try_from(i.as_u64().unwrap()).unwrap()])
        .collect();
    let added = breaking
        .iter()
        .find(|e| e["to"] == "src/entities/user/model.ts")
        .unwrap();
    assert_eq!(added["change"], "added");
    assert_eq!(added["from"], "src/features/checkout/index.ts");
    assert!(!added["breaks"].as_array().unwrap().is_empty(), "{added}");
}

#[tokio::test]
async fn drift_route_is_404_without_base() {
    let (status, _) = get("/api/drift").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
