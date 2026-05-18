use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::{Error as JsonRpcError, Result as JsonRpcResult};
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, DocumentLink, DocumentLinkOptions,
    DocumentLinkParams, Hover, HoverParams, HoverProviderCapability, InitializeParams,
    InitializeResult, MessageType, NumberOrString, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer};

use crate::debounce::Debouncer;
use crate::state::ServerState;
use crate::{document_links, hover};

const DEBOUNCE_MS: u64 = 50;

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
    pub state: Arc<Mutex<ServerState>>,
    pub debouncer: Arc<Debouncer>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        let mut state = self.state.lock().await;
        if let Err(err) = state.init(&params) {
            return Err(JsonRpcError::invalid_params(err.to_string()));
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_link_provider: Some(DocumentLinkOptions {
                    resolve_provider: Some(false),
                    work_done_progress_options: tower_lsp::lsp_types::WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "stratum-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: tower_lsp::lsp_types::InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "stratum-lsp ready")
            .await;
        self.spawn_plugin_watcher().await;
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        self.refresh(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let new_text = params
            .content_changes
            .into_iter()
            .last()
            .map(|c| c.text)
            .unwrap_or_default();
        {
            let mut state = self.state.lock().await;
            state.doc_text.insert(uri.clone(), new_text);
        }
        let backend = self.clone_handle();
        let uri_for_work = uri.clone();
        self.debouncer
            .schedule(
                uri,
                Duration::from_millis(DEBOUNCE_MS),
                move || async move {
                    backend.recompute(uri_for_work).await;
                },
            )
            .await;
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let state = self.state.lock().await;
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(diags) = state.last_diagnostics.get(&uri) else {
            return Ok(None);
        };
        let Some(d) = diags.iter().find(|d| d.range.start.line == pos.line) else {
            return Ok(None);
        };
        let slug = match &d.code {
            Some(NumberOrString::String(s)) => s.clone(),
            _ => return Ok(None),
        };
        let suggestion = state.last_suggestion_for(&uri, pos.line);
        Ok(Some(hover::hover_for(&slug, &d.message, suggestion)))
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> JsonRpcResult<Option<Vec<DocumentLink>>> {
        let state = self.state.lock().await;
        let uri = params.text_document.uri;
        let Some(diags) = state.last_diagnostics.get(&uri) else {
            return Ok(None);
        };
        Ok(Some(document_links::links_from(diags)))
    }
}

#[derive(Clone)]
struct BackendHandle {
    client: Client,
    state: Arc<Mutex<ServerState>>,
}

impl Backend {
    fn clone_handle(&self) -> BackendHandle {
        BackendHandle {
            client: self.client.clone(),
            state: Arc::clone(&self.state),
        }
    }

    async fn refresh(&self, uri: Url, text: String) {
        let handle = self.clone_handle();
        {
            let mut state = handle.state.lock().await;
            state.doc_text.insert(uri.clone(), text);
        }
        handle.recompute(uri).await;
    }

    async fn spawn_plugin_watcher(&self) {
        let Some(root) = self.state.lock().await.project_root.clone() else {
            return;
        };
        let rx = match stratum_plugins_rhai::watch_plugins(
            root.as_std_path(),
            Duration::from_millis(50),
        ) {
            Ok(rx) => rx,
            Err(err) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("plugin watcher failed to start: {err}"),
                    )
                    .await;
                return;
            }
        };
        let backend = self.clone_handle();
        let runtime = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            while let Ok(change) = rx.recv() {
                let backend = backend.clone();
                runtime.spawn(async move { backend.reload_after_plugin_change(change).await });
            }
        });
    }
}

impl BackendHandle {
    async fn reload_after_plugin_change(self, change: stratum_plugins_rhai::PluginChange) {
        let open_docs: Vec<Url> = {
            let mut state = self.state.lock().await;
            if state.rebuild().is_err() {
                return;
            }
            state.doc_text.keys().cloned().collect()
        };
        self.client
            .log_message(
                MessageType::INFO,
                format!("plugin reloaded: {}", change.path),
            )
            .await;
        for uri in open_docs {
            self.clone().recompute(uri).await;
        }
    }
}

impl BackendHandle {
    async fn recompute(self, uri: Url) {
        let (diagnostics, violations) = {
            let mut state = self.state.lock().await;
            if state.rebuild().is_err() {
                return;
            }
            let (diags, viols) = state.compute_diagnostics_for(&uri);
            state.last_diagnostics.insert(uri.clone(), diags.clone());
            state.last_violations.insert(uri.clone(), viols.clone());
            (diags, viols)
        };
        let _ = violations;
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}
