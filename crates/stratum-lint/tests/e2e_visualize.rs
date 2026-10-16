#![allow(clippy::unwrap_used)]

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
use tower::ServiceExt;

const FIXTURE: &str = "../../tests/fixtures/tiny-ts-violations";

fn router() -> Router {
    let root = Utf8Path::new(FIXTURE);
    let input = lint::build(root, &lint::load_config(root, None).unwrap()).unwrap();
    build_router(Arc::new(Api::new(input).unwrap()))
}

async fn get(uri: &str) -> (StatusCode, String) {
    let response = router()
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
