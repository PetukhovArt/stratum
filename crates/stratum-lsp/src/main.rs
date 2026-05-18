#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::{LspService, Server};

mod debounce;
mod diagnostics;
mod document_links;
mod hover;
mod server;
mod state;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| server::Backend {
        client,
        state: Arc::new(Mutex::new(state::ServerState::default())),
        debouncer: Arc::new(debounce::Debouncer::default()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
