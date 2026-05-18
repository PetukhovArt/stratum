#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::sync::mpsc::unbounded_channel;
use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

mod debounce;
mod diagnostics;
mod document_links;
mod hover;
mod log_layer;
mod server;
mod state;

#[tokio::main]
async fn main() {
    let (log_tx, mut log_rx) = unbounded_channel();
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,stratum=debug,stratum_lsp=debug"));
    let subscriber = tracing_subscriber::registry()
        .with(log_layer::LspLogLayer::new(log_tx))
        .with(filter);
    let _ = subscriber.try_init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        let backend = server::Backend {
            client: client.clone(),
            state: Arc::new(Mutex::new(state::ServerState::default())),
            debouncer: Arc::new(debounce::Debouncer::default()),
        };
        tokio::spawn(async move {
            while let Some(params) = log_rx.recv().await {
                client.log_message(params.typ, params.message).await;
            }
        });
        backend
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
