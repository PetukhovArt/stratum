//! Filesystem watcher for `.rhai` plugin files.
//!
//! Emits a [`PluginChange`] whenever a `.rhai` file under the watched
//! directory is modified or created. The host wires this into the lint
//! pipeline to refresh diagnostics without a process restart.

use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use camino::Utf8PathBuf;
use notify::{Event, EventKind, RecursiveMode, Watcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginChange {
    pub path: Utf8PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("notify init: {0}")]
    Init(#[from] notify::Error),
}

/// Watch `dir` recursively and emit [`PluginChange`] events for any `.rhai`
/// file that is created or modified.
///
/// Events are coalesced per path within `debounce`: a burst of OS events for
/// the same file (common on save) collapses to a single emitted change.
///
/// The returned [`Receiver`] is dropped to stop the watcher. The watcher
/// handle is moved into a background thread and dropped with the receiver.
pub fn watch_plugins(dir: &Path, debounce: Duration) -> Result<Receiver<PluginChange>, WatchError> {
    let (raw_tx, raw_rx) = channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(raw_tx)?;
    watcher.watch(dir, RecursiveMode::Recursive)?;

    let (out_tx, out_rx) = channel::<PluginChange>();
    std::thread::spawn(move || {
        let _watcher = watcher;
        let mut last_sent: std::collections::HashMap<Utf8PathBuf, Instant> =
            std::collections::HashMap::new();
        for res in raw_rx {
            let Ok(event) = res else { continue };
            if !is_change_event(event.kind) {
                continue;
            }
            for path in event.paths {
                let Ok(utf8) = Utf8PathBuf::from_path_buf(path) else {
                    continue;
                };
                if utf8.extension() != Some("rhai") {
                    continue;
                }
                let now = Instant::now();
                let recent = last_sent
                    .get(&utf8)
                    .is_some_and(|t| now.duration_since(*t) < debounce);
                if recent {
                    continue;
                }
                last_sent.insert(utf8.clone(), now);
                if send_change(&out_tx, PluginChange { path: utf8 }).is_err() {
                    return;
                }
            }
        }
    });

    Ok(out_rx)
}

fn is_change_event(kind: EventKind) -> bool {
    matches!(kind, EventKind::Create(_) | EventKind::Modify(_))
}

fn send_change(tx: &Sender<PluginChange>, change: PluginChange) -> Result<(), ()> {
    tx.send(change).map_err(|_| ())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn wait_event(rx: &Receiver<PluginChange>, timeout: Duration) -> Option<PluginChange> {
        rx.recv_timeout(timeout).ok()
    }

    #[test]
    fn emits_event_when_rhai_file_modified() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rule.rhai");
        std::fs::write(&file, "fn check() {}").unwrap();

        let rx = watch_plugins(dir.path(), Duration::from_millis(50)).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        std::fs::write(&file, "fn check() { 1 }").unwrap();

        let change = wait_event(&rx, Duration::from_secs(3)).expect("event arrives");
        assert!(change.path.as_str().ends_with("rule.rhai"));
    }

    #[test]
    fn ignores_non_rhai_files() {
        let dir = tempfile::tempdir().unwrap();
        let rx = watch_plugins(dir.path(), Duration::from_millis(50)).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        std::fs::write(dir.path().join("ignored.txt"), "hi").unwrap();
        assert!(wait_event(&rx, Duration::from_millis(500)).is_none());
    }
}
