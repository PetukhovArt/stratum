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
    build_router(Arc::new(Api::new(input, base, &[]).unwrap()))
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

const TINY: &str = "../../tests/fixtures/tiny-ts";

async fn tiny(uri: &str) -> (StatusCode, String) {
    get_from(router_at(Utf8Path::new(TINY), None), uri).await
}

async fn tiny_snapshot() -> serde_json::Value {
    serde_json::from_str(&tiny("/api/snapshot").await.1).unwrap()
}

fn id_at(snapshot: &serde_json::Value, list: &str, path: &str) -> u64 {
    snapshot[list]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["path"] == path)
        .unwrap()["id"]
        .as_u64()
        .unwrap()
}

async fn tiny_json(uri: &str) -> serde_json::Value {
    let (status, body) = tiny(uri).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    serde_json::from_str(&body).unwrap()
}

#[tokio::test]
async fn edge_route_resolves_each_imported_name_through_the_barrel_to_its_declaration() {
    let s = tiny_snapshot().await;
    let module = |path| id_at(&s, "modules", path);
    let container = |path| id_at(&s, "containers", path);
    let cart_to_user = serde_json::json!({ "imports": [{
        "file": "src/features/cart/cart.tsx", "line": 1, "type_only": false, "kind": "static",
        "names": [{ "name": "User", "type_only": false,
                    "decl": { "path": "src/entities/user/User.ts", "line": 3 } }],
    }]});

    let modules = format!(
        "/api/edge?from={}&to={}",
        module("src/features/cart/cart.tsx"),
        module("src/entities/user/index.ts")
    );
    assert_eq!(tiny_json(&modules).await, cart_to_user);
    let layers = format!(
        "/api/edge?from_container={}&to_container={}",
        container("src/features"),
        container("src/entities")
    );
    assert_eq!(tiny_json(&layers).await, cart_to_user);

    let renamed = format!(
        "/api/edge?from={}&to={}",
        module("src/app/main.ts"),
        module("src/features/cart/index.ts")
    );
    assert_eq!(
        tiny_json(&renamed).await["imports"][0]["names"],
        serde_json::json!([{ "name": "mountCart", "type_only": false,
                             "decl": { "path": "src/features/cart/cart.tsx", "line": 4 } }])
    );
}

#[tokio::test]
async fn edge_route_answers_no_imports_without_an_edge_and_400_without_both_ends() {
    let s = tiny_snapshot().await;
    let reverse = format!(
        "/api/edge?from={}&to={}",
        id_at(&s, "modules", "src/entities/user/index.ts"),
        id_at(&s, "modules", "src/features/cart/cart.tsx")
    );
    assert_eq!(
        tiny_json(&reverse).await,
        serde_json::json!({ "imports": [] })
    );
    for uri in [
        "/api/edge",
        "/api/edge?from=1",
        "/api/edge?from=a&to=1",
        "/api/edge?from=1&to_container=2",
        "/api/edge?from=1&to=2&from_container=1&to_container=2",
    ] {
        assert_eq!(tiny(uri).await.0, StatusCode::BAD_REQUEST, "{uri}");
    }
}

#[tokio::test]
async fn source_route_returns_the_lines_around_a_line_inside_the_root() {
    assert_eq!(
        tiny_json("/api/source?path=src/entities/user/User.ts&line=4&context=1").await,
        serde_json::json!({
            "path": "src/entities/user/User.ts", "start": 3, "total": 8,
            "lines": ["export interface User {", "  id: string;", "  name: string;"],
        })
    );
    let whole = tiny_json("/api/source?path=src/entities/user/index.ts").await;
    assert_eq!(whole["start"], 1);
    assert_eq!(
        whole["lines"],
        serde_json::json!([r#"export type { User } from "./User";"#])
    );
}

#[tokio::test]
async fn source_route_rejects_paths_outside_the_root_and_404s_a_missing_file() {
    let absolute = std::fs::canonicalize(format!("{TINY}/src/app/main.ts")).unwrap();
    for path in [
        "../tiny-ts-violations/src/app/main.ts".to_owned(),
        "src/../../tiny-ts/src/app/main.ts".to_owned(),
        absolute
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/"),
        "/src/app/main.ts".to_owned(),
    ] {
        let (status, _) = tiny(&format!("/api/source?path={path}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
    }
    let (status, _) = tiny("/api/source?path=src/app/missing.ts").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

const TINY_FEATURES: &str = "../../tests/fixtures/tiny-ts/features.config.jsonc";

fn tiny_features_api(selected: &[&str]) -> miette::Result<Api> {
    let root = Utf8Path::new(TINY);
    let config = lint::load_config(root, Some(Utf8Path::new(TINY_FEATURES))).unwrap();
    let selected: Vec<String> = selected.iter().map(|&s| s.to_owned()).collect();
    Api::new(lint::build(root, &config).unwrap(), None, &selected)
}

#[tokio::test]
async fn features_route_lists_each_feature_by_module_id_with_the_shared_ones_and_the_selection() {
    let router = build_router(Arc::new(tiny_features_api(&["cart", "legacy"]).unwrap()));
    let (status, body) = get_from(router.clone(), "/api/features").await;
    assert_eq!(status, StatusCode::OK);
    let s: serde_json::Value =
        serde_json::from_str(&get_from(router, "/api/snapshot").await.1).unwrap();
    let ids = |paths: &[&str]| {
        let mut ids: Vec<u64> = paths.iter().map(|p| id_at(&s, "modules", p)).collect();
        ids.sort_unstable();
        ids
    };
    let cart = [
        "src/entities/user/User.ts",
        "src/entities/user/index.ts",
        "src/features/cart/cart.tsx",
        "src/features/cart/index.ts",
        "src/shared/ui/Button.tsx",
        "src/shared/utils/format.ts",
    ];
    let boot = [&cart[..], &["src/app/main.ts", "src/shared/api/http.ts"]].concat();
    let shared = [&cart[..], &["src/shared/api/http.ts"]].concat();
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        parsed,
        serde_json::json!({
            "features": [
                { "id": "boot", "name": "boot",
                  "entry": ids(&["src/app/main.ts"]), "modules": ids(&boot) },
                { "id": "cart", "name": "Cart",
                  "entry": ids(&["src/features/cart/cart.tsx", "src/features/cart/index.ts"]),
                  "modules": ids(&cart) },
                { "id": "legacy", "name": "Legacy helper",
                  "entry": ids(&["src/legacy/old-helper.js"]),
                  "modules": ids(&["src/legacy/old-helper.js", "src/shared/api/http.ts"]) },
            ],
            "shared": ids(&shared),
            "selected": ["cart", "legacy"],
        })
    );
}

#[test]
fn an_unknown_selected_feature_is_an_error_naming_the_known_ones() {
    let err = tiny_features_api(&["cart", "chekout"]).unwrap_err();
    assert_eq!(
        err.to_string(),
        r#"unknown feature "chekout"; known: boot, cart, legacy"#
    );
}
