//! Watch mode for `stratum-lint --watch`.
//!
//! Manual smoke test: `cargo run -p stratum-lint -- --watch tests/fixtures/tiny-ts`,
//! then modify any file under that root; expect the lint output to re-render
//! after a 200ms debounce.

use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use camino::Utf8Path;
use notify::{Event, RecursiveMode, Watcher};

pub fn run<F: Fn() -> miette::Result<i32>>(root: &Utf8Path, lint: F) -> miette::Result<()> {
    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher =
        notify::recommended_watcher(tx).map_err(|e| miette::miette!("watcher init: {e}"))?;
    watcher
        .watch(root.as_std_path(), RecursiveMode::Recursive)
        .map_err(|e| miette::miette!("watcher start: {e}"))?;

    let _ = lint()?;

    let debounce = Duration::from_millis(200);
    let mut last_run = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(_event)) => {
                if last_run.elapsed() >= debounce {
                    let _ = lint()?;
                    last_run = Instant::now();
                }
            }
            Ok(Err(e)) => eprintln!("watch error: {e}"),
            Err(_) => continue,
        }
    }
}
