use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Query, State},
    http::{StatusCode, Uri, header},
    response::Response,
    routing::get,
};
use camino::Utf8Path;
use miette::IntoDiagnostic;
use rust_embed::RustEmbed;
use tokio::runtime::Builder;

use serde::Serialize;
use stratum_core::{
    edge::Edge, ids::ModuleId, severity::Severity, violation::SourceLocation, violation::Violation,
};
use stratum_engine::{Drift, EngineInput, Finding};

use crate::brief;
use crate::commands::{diff, lint};

#[derive(RustEmbed)]
#[folder = "../../frontend/stratum-visualizer-frontend/dist/"]
struct VisualizerAssets;

#[derive(Debug, Serialize)]
struct ApiViolation {
    rule: String,
    severity: Severity,
    message: String,
    file: String,
    location: SourceLocation,
    modules: Vec<ModuleId>,
    edge: Option<Edge>,
    fix: String,
}

#[derive(Debug)]
pub struct Api {
    input: EngineInput,
    findings: Vec<Finding>,
    snapshot: String,
    violations: String,
    /// `None` without `--base`: `/api/drift` answers 404.
    drift: Option<String>,
}

#[derive(Debug, Serialize)]
struct ApiDrift<'a> {
    base: ApiBase<'a>,
    markdown: String,
    /// Indices into `edges` of `Drift::breaking_imports`, in its order.
    breaking: Vec<usize>,
    #[serde(flatten)]
    drift: &'a Drift,
}

#[derive(Debug, Serialize)]
struct ApiBase<'a> {
    #[serde(rename = "ref")]
    reference: &'a str,
    sha: &'a str,
}

impl Api {
    /// `base` is a git ref: the drift of the working tree vs it is computed once, here.
    pub fn new(input: EngineInput, base: Option<&str>) -> miette::Result<Self> {
        let drift = base.map(|base| drift_json(&input, base)).transpose()?;
        let snapshot = input.snapshot().map_err(|e| miette::miette!("{e}"))?;
        let (api_violations, findings): (Vec<ApiViolation>, Vec<Finding>) = input
            .analyze_violations()
            .map_err(|e| miette::miette!("{e}"))?
            .into_iter()
            .map(|(v, f)| (api_violation(v, f.clone()), f))
            .unzip();
        Ok(Self {
            snapshot: serde_json::to_string(&snapshot).into_diagnostic()?,
            violations: serde_json::to_string(&api_violations).into_diagnostic()?,
            input,
            findings,
            drift,
        })
    }
}

fn drift_json(input: &EngineInput, base: &str) -> miette::Result<String> {
    let report = diff::compute(input.project_root.as_path(), base, &input.config)?;
    let api = ApiDrift {
        markdown: report.markdown(),
        breaking: report.drift.breaking_indices(),
        base: ApiBase {
            reference: report.base,
            sha: &report.sha,
        },
        drift: &report.drift,
    };
    serde_json::to_string(&api).into_diagnostic()
}

pub fn run(
    root: &Utf8Path,
    config: Option<&Utf8Path>,
    base: Option<&str>,
    open_browser: bool,
) -> miette::Result<()> {
    let api = Api::new(lint::build(root, &lint::load_config(root, config)?)?, base)?;
    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .into_diagnostic()?;
    rt.block_on(serve(api, open_browser))
}

fn api_violation(v: Violation, f: Finding) -> ApiViolation {
    ApiViolation {
        rule: f.rule,
        severity: f.severity,
        file: f.file,
        location: f.location,
        modules: v.modules,
        edge: v.edge,
        message: f.message,
        fix: f.fix,
    }
}

pub fn build_router(api: Arc<Api>) -> Router {
    Router::new()
        .route(
            "/api/snapshot",
            get(|State(api): State<Arc<Api>>| async move {
                text(StatusCode::OK, "application/json", api.snapshot.clone())
            }),
        )
        .route(
            "/api/violations",
            get(|State(api): State<Arc<Api>>| async move {
                text(StatusCode::OK, "application/json", api.violations.clone())
            }),
        )
        .route("/api/context", get(context_brief))
        .route(
            "/api/drift",
            get(|State(api): State<Arc<Api>>| async move {
                match &api.drift {
                    Some(drift) => text(StatusCode::OK, "application/json", drift.clone()),
                    None => text(
                        StatusCode::NOT_FOUND,
                        "text/plain; charset=utf-8",
                        "no drift: run `stratum-lint visualize --base REF`".into(),
                    ),
                }
            }),
        )
        .fallback(get(serve_asset))
        .with_state(api)
}

async fn context_brief(
    State(api): State<Arc<Api>>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    const PLAIN: &str = "text/plain; charset=utf-8";
    let Some(rel) = query.get("path") else {
        return text(StatusCode::BAD_REQUEST, PLAIN, "missing ?path=".into());
    };
    match brief::agent_brief(&api.input, &api.findings, rel) {
        Ok(brief) => text(StatusCode::OK, PLAIN, brief),
        Err(e) => text(StatusCode::BAD_REQUEST, PLAIN, e.to_string()),
    }
}

fn text(status: StatusCode, mime: &str, body: String) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(body))
        .unwrap_or_else(|_| internal_error())
}

async fn serve(api: Api, open_browser: bool) -> miette::Result<()> {
    let app = build_router(Arc::new(api));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .into_diagnostic()?;
    let addr = listener.local_addr().into_diagnostic()?;
    let url = format!("http://{addr}");
    eprintln!("stratum-visualizer ready at {url}");
    if open_browser {
        let _ = open::that(&url);
    }

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
