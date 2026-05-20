use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{StatusCode, Uri, header},
    response::Response,
    routing::get,
};
use camino::Utf8Path;
use miette::IntoDiagnostic;
use rust_embed::RustEmbed;
use stratum_graph::snapshot_of;
use tokio::runtime::Builder;

use crate::pipeline;
use crate::zero_config;

#[derive(RustEmbed)]
#[folder = "../../frontend/stratum-visualizer-frontend/dist/"]
struct VisualizerAssets;

pub fn run(root: &Utf8Path, port: u16) -> miette::Result<()> {
    let config_path = root.join("stratum.config.jsonc");
    let config = if config_path.exists() {
        stratum_config::parse_file(&config_path).map_err(|e| miette::miette!("{e}"))?
    } else {
        zero_config::infer(root)
    };
    let input = pipeline::build(root, &config).into_diagnostic()?;
    let snap = snapshot_of(&input.graph);
    let snap_json = serde_json::to_string(&snap).into_diagnostic()?;

    let violations = crate::engine::RuleEngine::run(&input);
    let violations_json = serde_json::to_string(&violations).into_diagnostic()?;

    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .into_diagnostic()?;
    rt.block_on(serve(snap_json, violations_json, port))
}

pub fn build_router(snapshot: &Arc<String>, violations: &Arc<String>) -> Router {
    let snap = Arc::clone(snapshot);
    let snapshot_route = get(move || {
        let snap = Arc::clone(&snap);
        async move {
            Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(snap.as_ref().clone()))
                .unwrap_or_else(|_| internal_error())
        }
    });
    let vio = Arc::clone(violations);
    let violations_route = get(move || {
        let vio = Arc::clone(&vio);
        async move {
            Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(vio.as_ref().clone()))
                .unwrap_or_else(|_| internal_error())
        }
    });
    Router::new()
        .route("/api/snapshot", snapshot_route)
        .route("/api/violations", violations_route)
        .fallback(get(serve_asset))
}

async fn serve(snapshot_json: String, violations_json: String, port: u16) -> miette::Result<()> {
    let snapshot = Arc::new(snapshot_json);
    let violations = Arc::new(violations_json);
    let app = build_router(&snapshot, &violations);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .into_diagnostic()?;
    let addr = listener.local_addr().into_diagnostic()?;
    let url = format!("http://{addr}");
    eprintln!("stratum-visualizer ready at {url}");
    let _ = open::that(&url);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .into_diagnostic()?;
    Ok(())
}

async fn serve_asset(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match VisualizerAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap_or_else(|_| internal_error())
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap_or_else(|_| internal_error()),
    }
}

fn internal_error() -> Response {
    let mut resp = Response::new(Body::from("internal error"));
    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    resp
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn embedded_index_html_present() {
        let Some(asset) = VisualizerAssets::get("index.html") else {
            unreachable!("index.html must be embedded by build.rs / rust-embed")
        };
        let body = std::str::from_utf8(&asset.data).unwrap();
        assert!(body.contains("Stratum Visualizer"));
    }
}
