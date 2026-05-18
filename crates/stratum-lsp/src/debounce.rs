use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tower_lsp::lsp_types::Url;

#[derive(Debug, Default)]
pub struct Debouncer {
    handles: Mutex<HashMap<Url, JoinHandle<()>>>,
}

impl Debouncer {
    pub async fn schedule<F, Fut>(&self, uri: Url, wait: Duration, work: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut map = self.handles.lock().await;
        if let Some(h) = map.remove(&uri) {
            h.abort();
        }
        let handle = tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            work().await;
        });
        map.insert(uri, handle);
    }
}
