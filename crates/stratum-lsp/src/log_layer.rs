//! Tracing layer that forwards events to an LSP `window/logMessage` channel.

use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;
use tower_lsp::lsp_types::{LogMessageParams, MessageType};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

pub struct LspLogLayer {
    tx: Arc<UnboundedSender<LogMessageParams>>,
}

impl LspLogLayer {
    pub fn new(tx: UnboundedSender<LogMessageParams>) -> Self {
        Self { tx: Arc::new(tx) }
    }
}

impl<S: Subscriber> Layer<S> for LspLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let typ = match *event.metadata().level() {
            Level::ERROR => MessageType::ERROR,
            Level::WARN => MessageType::WARNING,
            Level::INFO => MessageType::INFO,
            _ => MessageType::LOG,
        };
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        let _ = self.tx.send(LogMessageParams {
            typ,
            message: visitor.0,
        });
    }
}

struct MessageVisitor(String);

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}").trim_matches('"').to_string();
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::unbounded_channel;
    use tracing_subscriber::prelude::*;

    #[tokio::test]
    async fn forwards_info_event_as_info_log_message() {
        let (tx, mut rx) = unbounded_channel();
        let subscriber = tracing_subscriber::registry().with(LspLogLayer::new(tx));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("hello-stratum");
        });
        let msg = rx.recv().await.expect("log message expected");
        assert_eq!(msg.typ, MessageType::INFO);
        assert!(msg.message.contains("hello-stratum"));
    }

    #[tokio::test]
    async fn maps_levels_to_message_types() {
        let (tx, mut rx) = unbounded_channel();
        let subscriber = tracing_subscriber::registry().with(LspLogLayer::new(tx));
        tracing::subscriber::with_default(subscriber, || {
            tracing::error!("oops");
            tracing::warn!("careful");
        });
        let m1 = rx.recv().await.unwrap();
        let m2 = rx.recv().await.unwrap();
        assert_eq!(m1.typ, MessageType::ERROR);
        assert_eq!(m2.typ, MessageType::WARNING);
    }
}
