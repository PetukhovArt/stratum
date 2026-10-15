#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use tokio::sync::mpsc::unbounded_channel;
use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

mod analysis;
mod diagnostics;
mod document_links;
mod exit;
mod hover;
mod log_layer;
mod paths;
mod server;
mod state;
mod watch;

#[tokio::main]
async fn main() {
    let (log_tx, mut log_rx) = unbounded_channel();
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,stratum_lsp=info"));
    let subscriber = tracing_subscriber::registry()
        .with(log_layer::LspLogLayer::new(log_tx))
        .with(filter);
    let _ = subscriber.try_init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        let backend = server::Backend::new(client.clone());
        tokio::spawn(async move {
            while let Some(params) = log_rx.recv().await {
                client.log_message(params.typ, params.message).await;
            }
        });
        backend
    });
    Server::new(stdin, stdout, socket)
        .serve(exit::ExitOnNotification::new(service))
        .await;
}
