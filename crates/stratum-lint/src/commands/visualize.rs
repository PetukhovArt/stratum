use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Query, State},
    http::{StatusCode, Uri, header},
    response::Response,
    routing::get,
};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use miette::IntoDiagnostic;
use rust_embed::RustEmbed;
use tokio::runtime::Builder;

use serde::Serialize;
use stratum_core::{
    edge::{Edge, EdgeKind},
    ids::{ContainerId, ModuleId},
    severity::Severity,
    violation::SourceLocation,
    violation::Violation,
};
use stratum_engine::{Drift, EngineInput, Finding, paths};
use stratum_graph::{CompoundGraph, crossing_names};

use crate::brief;
use crate::commands::{diff, features, lint};

mod site;

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
    features: String,
    /// `None` without `systems` in the config: `/api/systems` answers 404.
    systems: Option<String>,
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
    /// `selected` are the feature ids the filter opens with; an unknown one is an error that
    /// names the known ids.
    pub fn new(
        input: EngineInput,
        base: Option<&str>,
        selected: &[String],
    ) -> miette::Result<Self> {
        let drift = base.map(|base| drift_json(&input, base)).transpose()?;
        let features = features_json(&input, selected)?;
        let systems = input
            .systems()
            .map_err(|e| miette::miette!("{e}"))?
            .map(|map| serde_json::to_string(&map))
            .transpose()
            .into_diagnostic()?;
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
            features,
            systems,
            input,
            findings,
            drift,
        })
    }
}

#[derive(Debug, Serialize)]
struct ApiFeatures<'a> {
    #[serde(flatten)]
    listing: features::Listing<ModuleId>,
    selected: &'a [String],
}

fn features_json(input: &EngineInput, selected: &[String]) -> miette::Result<String> {
    let features = input.features().map_err(|e| miette::miette!("{e}"))?;
    features::ensure_known(&features, selected)?;
    let api = ApiFeatures {
        listing: features::listing(&features, |m| m),
        selected,
    };
    serde_json::to_string(&api).into_diagnostic()
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
    selected: &[String],
    open_browser: bool,
) -> miette::Result<()> {
    let input = lint::build(root, &lint::load_config(root, config)?)?;
    let api = Api::new(input, base, selected)?;
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
        .route(
            "/api/features",
            get(|State(api): State<Arc<Api>>| async move {
                text(StatusCode::OK, "application/json", api.features.clone())
            }),
        )
        .route(
            "/api/systems",
            get(|State(api): State<Arc<Api>>| async move {
                match &api.systems {
                    Some(systems) => text(StatusCode::OK, "application/json", systems.clone()),
                    None => text(
                        StatusCode::NOT_FOUND,
                        PLAIN,
                        "no systems: declare `systems` in the config".into(),
                    ),
                }
            }),
        )
        .route("/api/context", get(context_brief))
        .route("/api/edge", get(edge_contract))
        .route("/api/source", get(source_window))
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
    let Some(rel) = query.get("path") else {
        return text(StatusCode::BAD_REQUEST, PLAIN, "missing ?path=".into());
    };
    match brief::agent_brief(&api.input, &api.findings, rel) {
        Ok(brief) => text(StatusCode::OK, PLAIN, brief),
        Err(e) => text(StatusCode::BAD_REQUEST, PLAIN, e.to_string()),
    }
}

#[derive(Debug, Serialize)]
struct ApiEdge {
    imports: Vec<ApiImport>,
}

#[derive(Debug, Serialize)]
struct ApiImport {
    file: String,
    line: u32,
    type_only: bool,
    kind: EdgeKind,
    names: Vec<ApiName>,
}

#[derive(Debug, Serialize)]
struct ApiName {
    name: String,
    type_only: bool,
    decl: Option<ApiDecl>,
}

#[derive(Debug, Serialize)]
struct ApiDecl {
    path: String,
    line: u32,
}

#[derive(Debug, Clone, Copy)]
enum EdgeEnd {
    Module(ModuleId),
    /// Its modules and those of every container nested in it.
    Container(ContainerId),
}

impl EdgeEnd {
    fn modules(self, graph: &CompoundGraph) -> HashSet<ModuleId> {
        match self {
            Self::Module(id) => HashSet::from([id]),
            Self::Container(c) => graph
                .modules
                .keys()
                .filter(|&&m| graph.containers_of(m).any(|held| held.id == c))
                .copied()
                .collect(),
        }
    }
}

async fn edge_contract(
    State(api): State<Arc<Api>>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let Some((from, to)) = edge_ends(&query) else {
        return text(
            StatusCode::BAD_REQUEST,
            PLAIN,
            "pass ?from=&to= module ids or ?from_container=&to_container= container ids".into(),
        );
    };
    json(&edge_imports(&api.input, from, to))
}

fn edge_ends(query: &HashMap<String, String>) -> Option<(EdgeEnd, EdgeEnd)> {
    let id = |key: &str| query.get(key).and_then(|v| v.parse::<u32>().ok());
    let given = |keys: [&str; 2]| keys.iter().any(|k| query.contains_key(*k));
    let modules = given(["from", "to"]).then(|| {
        Some((
            EdgeEnd::Module(ModuleId::new(id("from")?)),
            EdgeEnd::Module(ModuleId::new(id("to")?)),
        ))
    });
    let containers = given(["from_container", "to_container"]).then(|| {
        Some((
            EdgeEnd::Container(ContainerId::new(id("from_container")?)),
            EdgeEnd::Container(ContainerId::new(id("to_container")?)),
        ))
    });
    match (modules, containers) {
        (Some(ends), None) | (None, Some(ends)) => ends,
        _ => None,
    }
}

fn edge_imports(input: &EngineInput, from: EdgeEnd, to: EdgeEnd) -> ApiEdge {
    let graph = &input.graph;
    let targets = to.modules(graph);
    let path_of = |id: ModuleId| graph.modules.get(&id).map(|m| input.relative(&m.path));
    let mut imports: Vec<(SourceLocation, ApiImport)> = from
        .modules(graph)
        .into_iter()
        .filter_map(|id| graph.modules.get(&id))
        .flat_map(|m| {
            let file = input.relative(&m.path);
            m.imports
                .iter()
                .filter(|i| i.target.is_some_and(|t| targets.contains(&t)))
                .map(move |i| (file.clone(), i))
        })
        .map(|(file, i)| {
            let names = crossing_names(graph, i)
                .into_iter()
                .map(|n| ApiName {
                    name: n.name,
                    type_only: n.type_only,
                    decl: n.decl.and_then(|d| {
                        Some(ApiDecl {
                            path: path_of(d.module)?,
                            line: d.location.line,
                        })
                    }),
                })
                .collect();
            let import = ApiImport {
                file,
                line: i.location.line,
                type_only: i.type_only,
                kind: i.kind,
                names,
            };
            (i.location, import)
        })
        .collect();
    imports.sort_by(|(a_at, a), (b_at, b)| {
        (&a.file, a_at.line, a_at.column).cmp(&(&b.file, b_at.line, b_at.column))
    });
    ApiEdge {
        imports: imports.into_iter().map(|(_, i)| i).collect(),
    }
}

const SOURCE_CONTEXT: usize = 20;
const SOURCE_CONTEXT_CAP: usize = 200;
const SOURCE_WHOLE_FILE_CAP: usize = 2000;

#[derive(Debug, Serialize)]
struct ApiSource {
    path: String,
    start: usize,
    lines: Vec<String>,
    total: usize,
}

async fn source_window(
    State(api): State<Arc<Api>>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    match read_source(&api.input, &query) {
        Ok(source) => json(&source),
        Err((status, message)) => text(status, PLAIN, message),
    }
}

fn read_source(
    input: &EngineInput,
    query: &HashMap<String, String>,
) -> Result<ApiSource, (StatusCode, String)> {
    let bad = |message: String| (StatusCode::BAD_REQUEST, message);
    let number = |key: &str| {
        query
            .get(key)
            .map(|v| {
                v.parse::<usize>()
                    .map_err(|_| bad(format!("?{key}= is not a number")))
            })
            .transpose()
    };
    let rel = query
        .get("path")
        .ok_or_else(|| bad("missing ?path=".into()))?;
    let line = number("line")?;
    let context = number("context")?
        .unwrap_or(SOURCE_CONTEXT)
        .min(SOURCE_CONTEXT_CAP);
    let SourceFile { path, lines: all } = source_lines(input, rel)?;
    let (start, count) = match line {
        Some(0) => return Err(bad("?line= starts at 1".into())),
        Some(n) => {
            let start = n.saturating_sub(context).max(1);
            (start, n + context + 1 - start)
        }
        None => (1, SOURCE_WHOLE_FILE_CAP),
    };
    let total = all.len();
    Ok(ApiSource {
        path,
        start,
        lines: all.into_iter().skip(start - 1).take(count).collect(),
        total,
    })
}

#[derive(Debug, Serialize)]
struct SourceFile {
    /// Canonical and relative: `rel/path` under ROOT, `system-id:rel/path` under a system root.
    path: String,
    lines: Vec<String>,
}

/// The lines of `rel`, a path under ROOT or `system-id:rel/path` under that system's root.
fn source_lines(input: &EngineInput, rel: &str) -> Result<SourceFile, (StatusCode, String)> {
    let (system, root, inner) = match rel.split_once(':') {
        Some((id, inner)) if let Some(root) = input.system_root(id) => (Some(id), root, inner),
        _ => (None, input.project_root.clone(), rel),
    };
    let whose = system.map_or_else(
        || "the project root".to_owned(),
        |id| format!("the root of system {id:?}"),
    );
    let (file, root) = source_file(&root, inner, &whose)?;
    let bytes =
        std::fs::read(&file).map_err(|e| (StatusCode::BAD_REQUEST, format!("{rel}: {e}")))?;
    let path = paths::relative(file.as_std_path(), &root);
    Ok(SourceFile {
        path: system.map_or_else(|| path.clone(), |id| format!("{id}:{path}")),
        lines: String::from_utf8_lossy(&bytes)
            .lines()
            .map(str::to_owned)
            .collect(),
    })
}

/// The file at `rel` under `root`, and `root`, both canonical; `whose` names the root in errors.
fn source_file(
    root: &Utf8Path,
    rel: &str,
    whose: &str,
) -> Result<(Utf8PathBuf, Utf8PathBuf), (StatusCode, String)> {
    let outside = || (StatusCode::BAD_REQUEST, format!("{rel} is outside {whose}"));
    let escapes = Utf8Path::new(rel).components().any(|c| {
        matches!(
            c,
            Utf8Component::Prefix(_) | Utf8Component::RootDir | Utf8Component::ParentDir
        )
    });
    if escapes {
        return Err(outside());
    }
    let file = paths::canonicalize(&root.join(rel)).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, format!("{rel}: no such file")),
        _ => (StatusCode::BAD_REQUEST, format!("{rel}: {e}")),
    })?;
    let root = paths::canonicalize(root).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if file.starts_with(&root) {
        Ok((file, root))
    } else {
        Err(outside())
    }
}

const PLAIN: &str = "text/plain; charset=utf-8";

fn json(body: &impl Serialize) -> Response {
    match serde_json::to_string(body) {
        Ok(body) => text(StatusCode::OK, "application/json", body),
        Err(_) => internal_error(),
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
