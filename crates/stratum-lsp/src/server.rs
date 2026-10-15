use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tower_lsp::jsonrpc::{Error as JsonRpcError, Result as JsonRpcResult};
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, DocumentLink, DocumentLinkOptions, DocumentLinkParams, Hover,
    HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
    MessageType, NumberOrString, SaveOptions, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Url, WorkDoneProgressOptions,
};
use tower_lsp::{Client, LanguageServer};

use crate::analysis;
use crate::paths::{path_key, uri_to_path};
use crate::state::{ServerState, resolve_root};
use crate::watch::InputWatcher;
use crate::{document_links, hover};

const DEBOUNCE: Duration = Duration::from_millis(50);

#[derive(Debug, Clone)]
pub struct Backend {
    client: Client,
    state: Arc<Mutex<ServerState>>,
    requests: UnboundedSender<()>,
    pending: Arc<Mutex<Option<UnboundedReceiver<()>>>>,
    watcher: Arc<std::sync::Mutex<InputWatcher>>,
}

impl Backend {
    #[must_use]
    pub fn new(client: Client) -> Self {
        let (requests, pending) = unbounded_channel();
        Self {
            client,
            state: Arc::new(Mutex::new(ServerState::default())),
            requests,
            pending: Arc::new(Mutex::new(Some(pending))),
            watcher: Arc::new(std::sync::Mutex::new(InputWatcher::default())),
        }
    }

    fn request_analysis(&self) {
        let _ = self.requests.send(());
    }

    async fn run_analyses(self, mut requests: UnboundedReceiver<()>) {
        while requests.recv().await.is_some() {
            tokio::time::sleep(DEBOUNCE).await;
            while requests.try_recv().is_ok() {}
            self.analyze_once().await;
        }
    }

    async fn analyze_once(&self) {
        let root = {
            let mut state = self.state.lock().await;
            state.analysis_started = Some(SystemTime::now());
            state.root.clone()
        };
        let Some(root) = root else { return };
        let started = Instant::now();
        let analysis = match tokio::task::spawn_blocking(move || analysis::analyze(&root)).await {
            Ok(a) => a,
            Err(err) => {
                tracing::error!("analysis crashed: {err}");
                return;
            }
        };
        let watch_result = self
            .watcher
            .lock()
            .map_err(|_| "watcher lock poisoned".to_string())
            .and_then(|mut w| {
                w.watch(&analysis.inputs, self.requests.clone())
                    .map_err(|e| e.to_string())
            });
        if let Err(err) = watch_result {
            tracing::warn!("config watcher failed: {err}");
        }
        match analysis.outcome {
            Ok(diagnostics) => {
                let publishes: Vec<_> = {
                    let mut state = self.state.lock().await;
                    state.diagnostics = Some(diagnostics);
                    state.last_error = None;
                    state
                        .open
                        .iter()
                        .map(|(key, uri)| (uri.clone(), state.diagnostics_for(key)))
                        .collect()
                };
                tracing::info!("analysis done in {} ms", started.elapsed().as_millis());
                for (uri, diags) in publishes {
                    self.client.publish_diagnostics(uri, diags, None).await;
                }
            }
            Err(err) => {
                let message = format!("stratum: {err}");
                tracing::error!("{message}");
                let repeated = {
                    let mut state = self.state.lock().await;
                    state.last_error.replace(message.clone()).as_deref() == Some(message.as_str())
                };
                if !repeated {
                    self.client.show_message(MessageType::ERROR, message).await;
                }
            }
        }
    }

    async fn touch(&self, uri: Url, publish_cached: bool) {
        let Some(path) = uri_to_path(&uri) else {
            return;
        };
        let key = path_key(&path);
        let (needs_analysis, previous) = {
            let mut state = self.state.lock().await;
            state.open.insert(key.clone(), uri.clone());
            (
                state.is_stale(&path),
                state
                    .diagnostics
                    .is_some()
                    .then(|| state.diagnostics_for(&key)),
            )
        };
        if needs_analysis {
            self.request_analysis();
        } else if let (true, Some(diags)) = (publish_cached, previous) {
            self.client.publish_diagnostics(uri, diags, None).await;
        }
    }

    async fn diagnostic_at(&self, uri: &Url, line: u32) -> Option<(String, String)> {
        let key = path_key(&uri_to_path(uri)?);
        let state = self.state.lock().await;
        state
            .diagnostics_for(&key)
            .into_iter()
            .find(|d| d.range.start.line == line)
            .and_then(|d| match d.code {
                Some(NumberOrString::String(slug)) => Some((slug, d.message)),
                _ => None,
            })
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        let root = resolve_root(&params)
            .ok_or_else(|| JsonRpcError::invalid_params("no usable workspace folder or rootUri"))?;
        self.state.lock().await.root = Some(root);
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_link_provider: Some(DocumentLinkOptions {
                    resolve_provider: Some(false),
                    work_done_progress_options: WorkDoneProgressOptions {
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

    async fn initialized(&self, _: InitializedParams) {
        let Some(requests) = self.pending.lock().await.take() else {
            return;
        };
        tokio::spawn(self.clone().run_analyses(requests));
        self.request_analysis();
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.touch(params.text_document.uri, true).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.touch(params.text_document.uri, false).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.touch(params.text_document.uri, false).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(path) = uri_to_path(&uri) {
            self.state.lock().await.open.remove(&path_key(&path));
        }
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let pos = params.text_document_position_params;
        Ok(self
            .diagnostic_at(&pos.text_document.uri, pos.position.line)
            .await
            .map(|(slug, message)| hover::hover_for(&slug, &message)))
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> JsonRpcResult<Option<Vec<DocumentLink>>> {
        let Some(path) = uri_to_path(&params.text_document.uri) else {
            return Ok(None);
        };
        let state = self.state.lock().await;
        Ok(Some(document_links::links_from(
            &state.diagnostics_for(&path_key(&path)),
        )))
    }
}
