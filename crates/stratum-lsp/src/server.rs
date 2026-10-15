use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tower_lsp::jsonrpc::{Error as JsonRpcError, Result as JsonRpcResult};
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWatchedFilesRegistrationOptions, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentLink, DocumentLinkOptions, DocumentLinkParams,
    FileChangeType, FileSystemWatcher, GlobPattern, Hover, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, MessageType, NumberOrString,
    Registration, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, Url, WorkDoneProgressOptions,
};
use tower_lsp::{Client, LanguageServer};

use crate::analysis::{self, Project, Text};
use crate::paths::{path_key, uri_to_path};
use crate::state::{OpenDoc, ServerState, resolve_root};
use crate::watch::InputWatcher;
use crate::{document_links, hover};

const DEBOUNCE: Duration = Duration::from_millis(50);
const WATCHED_SOURCES: &str = "**/*.{ts,tsx,js,jsx,mjs,cjs,vue}";

#[derive(Debug)]
pub enum Job {
    Rebuild,
    Edit(String, PathBuf),
}

#[derive(Debug, Default)]
struct Batch {
    rebuild: bool,
    edits: BTreeMap<String, PathBuf>,
}

impl Batch {
    fn add(&mut self, job: Job) {
        match job {
            Job::Rebuild => self.rebuild = true,
            Job::Edit(key, path) => {
                self.edits.insert(key, path);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Backend {
    client: Client,
    state: Arc<Mutex<ServerState>>,
    jobs: UnboundedSender<Job>,
    pending: Arc<Mutex<Option<UnboundedReceiver<Job>>>>,
    watcher: Arc<std::sync::Mutex<InputWatcher>>,
    watch_sources: Arc<std::sync::atomic::AtomicBool>,
}

impl Backend {
    #[must_use]
    pub fn new(client: Client) -> Self {
        let (jobs, pending) = unbounded_channel();
        Self {
            client,
            state: Arc::new(Mutex::new(ServerState::default())),
            jobs,
            pending: Arc::new(Mutex::new(Some(pending))),
            watcher: Arc::new(std::sync::Mutex::new(InputWatcher::default())),
            watch_sources: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn schedule(&self, job: Job) {
        let _ = self.jobs.send(job);
    }

    async fn run_analyses(self, mut jobs: UnboundedReceiver<Job>) {
        let mut project = None;
        while let Some(job) = jobs.recv().await {
            tokio::time::sleep(DEBOUNCE).await;
            let mut batch = Batch::default();
            batch.add(job);
            while let Ok(job) = jobs.try_recv() {
                batch.add(job);
            }
            project = self.analyze(project, batch).await;
        }
    }

    async fn analyze(&self, project: Option<Project>, batch: Batch) -> Option<Project> {
        let (root, edits, open) = {
            let state = self.state.lock().await;
            let edits: Vec<(PathBuf, Text)> = batch
                .edits
                .into_iter()
                .map(|(key, path)| {
                    let text = state
                        .open
                        .get(&key)
                        .map_or(Text::Disk, |d| Text::Buffer(d.text.clone()));
                    (path, text)
                })
                .collect();
            let open: Vec<(PathBuf, String)> = state
                .open
                .values()
                .map(|d| (d.path.clone(), d.text.clone()))
                .collect();
            (state.root.clone(), edits, open)
        };
        let Some(root) = root else { return project };
        let started = Instant::now();
        let rebuild = batch.rebuild;
        let run = match tokio::task::spawn_blocking(move || {
            analysis::run(project, &root, rebuild, &edits, &open)
        })
        .await
        {
            Ok(run) => run,
            Err(err) => {
                tracing::error!("analysis crashed: {err}");
                return None;
            }
        };
        if let Some(inputs) = &run.inputs {
            self.watch_inputs(inputs);
        }
        match run.outcome {
            Ok(Some(diagnostics)) => {
                let publishes = {
                    let mut state = self.state.lock().await;
                    state.diagnostics = Some(diagnostics);
                    state.last_error = None;
                    state.unpublished()
                };
                tracing::info!(
                    "analysis ({}) done in {} ms",
                    if run.inputs.is_some() {
                        "full"
                    } else {
                        "incremental"
                    },
                    started.elapsed().as_millis()
                );
                for (uri, diags) in publishes {
                    self.client.publish_diagnostics(uri, diags, None).await;
                }
            }
            Ok(None) => {}
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
        run.project
    }

    fn watch_inputs(&self, inputs: &[PathBuf]) {
        let result = self
            .watcher
            .lock()
            .map_err(|_| "watcher lock poisoned".to_string())
            .and_then(|mut w| {
                w.watch(inputs, self.jobs.clone())
                    .map_err(|e| e.to_string())
            });
        if let Err(err) = result {
            tracing::warn!("config watcher failed: {err}");
        }
    }

    async fn register_source_watcher(&self) {
        let options = DidChangeWatchedFilesRegistrationOptions {
            watchers: vec![FileSystemWatcher {
                glob_pattern: GlobPattern::String(WATCHED_SOURCES.into()),
                kind: None,
            }],
        };
        let registration = Registration {
            id: "stratum-sources".into(),
            method: "workspace/didChangeWatchedFiles".into(),
            register_options: serde_json::to_value(options).ok(),
        };
        if let Err(err) = self.client.register_capability(vec![registration]).await {
            tracing::warn!("could not watch source files: {err}");
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
        let dynamic_watch = params
            .capabilities
            .workspace
            .and_then(|w| w.did_change_watched_files)
            .and_then(|w| w.dynamic_registration)
            .unwrap_or(false);
        self.watch_sources
            .store(dynamic_watch, std::sync::atomic::Ordering::Relaxed);
        self.state.lock().await.root = Some(root);
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
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
        let Some(jobs) = self.pending.lock().await.take() else {
            return;
        };
        tokio::spawn(self.clone().run_analyses(jobs));
        self.schedule(Job::Rebuild);
        if self
            .watch_sources
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            let backend = self.clone();
            tokio::spawn(async move { backend.register_source_watcher().await });
        }
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        let Some(path) = uri_to_path(&doc.uri) else {
            return;
        };
        let key = path_key(&path);
        let cached = {
            let mut state = self.state.lock().await;
            let cached = state
                .diagnostics
                .is_some()
                .then(|| state.diagnostics_for(&key));
            state.open.insert(
                key.clone(),
                OpenDoc {
                    uri: doc.uri.clone(),
                    path: path.clone(),
                    text: doc.text,
                    published: cached.clone(),
                },
            );
            cached
        };
        if let Some(diags) = cached {
            self.client.publish_diagnostics(doc.uri, diags, None).await;
        }
        self.schedule(Job::Edit(key, path));
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let Some(path) = uri_to_path(&params.text_document.uri) else {
            return;
        };
        let Some(text) = params
            .content_changes
            .into_iter()
            .next_back()
            .map(|c| c.text)
        else {
            return;
        };
        let key = path_key(&path);
        if let Some(doc) = self.state.lock().await.open.get_mut(&key) {
            doc.text = text;
        }
        self.schedule(Job::Edit(key, path));
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(path) = uri_to_path(&uri) {
            let key = path_key(&path);
            self.state.lock().await.open.remove(&key);
            self.schedule(Job::Edit(key, path));
        }
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let state = self.state.lock().await;
        for change in params.changes {
            let Some(path) = uri_to_path(&change.uri) else {
                continue;
            };
            let key = path_key(&path);
            if change.typ != FileChangeType::CHANGED {
                self.schedule(Job::Rebuild);
            } else if !state.open.contains_key(&key) {
                self.schedule(Job::Edit(key, path));
            }
        }
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
