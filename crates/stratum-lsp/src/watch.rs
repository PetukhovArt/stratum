use std::collections::BTreeSet;
use std::path::PathBuf;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc::UnboundedSender;

use crate::paths::path_key;

#[derive(Default)]
pub struct InputWatcher {
    keys: BTreeSet<String>,
    watcher: Option<RecommendedWatcher>,
}

impl std::fmt::Debug for InputWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputWatcher")
            .field("keys", &self.keys)
            .finish_non_exhaustive()
    }
}

impl InputWatcher {
    pub fn watch(
        &mut self,
        files: &[PathBuf],
        on_change: UnboundedSender<()>,
    ) -> notify::Result<()> {
        let keys: BTreeSet<String> = files.iter().map(|f| path_key(f)).collect();
        if self.watcher.is_some() && keys == self.keys {
            return Ok(());
        }
        let dirs: BTreeSet<PathBuf> = files
            .iter()
            .filter_map(|f| f.parent())
            .filter(|d| d.is_dir())
            .map(PathBuf::from)
            .collect();
        let matched = keys.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            if matches!(event.kind, EventKind::Access(_)) {
                return;
            }
            if event.paths.iter().any(|p| matched.contains(&path_key(p))) {
                let _ = on_change.send(());
            }
        })?;
        for dir in &dirs {
            watcher.watch(dir, RecursiveMode::NonRecursive)?;
        }
        self.watcher = Some(watcher);
        self.keys = keys;
        Ok(())
    }
}
