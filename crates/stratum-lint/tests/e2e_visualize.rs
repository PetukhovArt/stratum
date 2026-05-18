#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use stratum_lint::commands::visualize::build_router;
use tower::ServiceExt;

#[tokio::test]
async fn snapshot_route_returns_json_payload() {
    let snapshot = Arc::new(
        r#"{"version":1,"modules":[],"containers":[],"layers":[],"edges":[]}"#.to_string(),
    );
    let router = build_router(&snapshot);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/snapshot")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["version"], 1);
}

#[tokio::test]
async fn root_path_serves_embedded_index_html() {
    let snapshot = Arc::new(String::from(r#"{"version":1}"#));
    let router = build_router(&snapshot);

    let response = router
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&body).unwrap();
    assert!(text.contains("Stratum Visualizer"));
}

#[tokio::test]
async fn unknown_asset_path_returns_404() {
    let snapshot = Arc::new(String::from(r#"{"version":1}"#));
    let router = build_router(&snapshot);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/no-such-asset.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
