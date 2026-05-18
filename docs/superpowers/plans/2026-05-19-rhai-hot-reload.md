# Rhai Plugin Hot-Reload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a `.rhai` plugin file changes on disk, the lint pipeline (CLI watch-mode and LSP) re-compiles the script and re-runs the affected rules within ~50 ms of the save — without restarting the process.

**Architecture:** The Rhai plugin loader gets a `notify`-backed watcher that emits "plugin changed" events. Each plugin's compiled `AST` is held behind a Salsa input; the watcher updates the input via `set_plugin_source`, which invalidates downstream queries. The watcher is opt-in via a `--watch-plugins` CLI flag (lint CLI) and always-on in LSP. A perf-gate test asserts reload latency.

**Tech Stack:** Rust, `notify` crate (already a dep of `stratum-lint`), `salsa` (existing), `rhai` (existing in `stratum-plugins-rhai`).

**Depends on:** Nothing strictly. Independent of the rendering and web-client plans.

---

### Task 1: Move plugin source into a Salsa input

**Files:**
- Modify: `crates/stratum-core/src/db.rs` (declare the Salsa input + helper)
- Modify: `crates/stratum-plugins-rhai/src/loader.rs` (read the input, not the file)
- Modify: `crates/stratum-plugins-rhai/src/lib.rs` (re-export)

Today the loader presumably reads `.rhai` files directly on each lint run. To make Salsa drive invalidation, the *source code* must live behind a Salsa input — then Salsa knows downstream queries (the compiled AST, the rule list, the per-file diagnostics) depend on it.

- [ ] **Step 1: Read the current loader to confirm the call shape**

```bash
cat crates/stratum-plugins-rhai/src/loader.rs
cat crates/stratum-plugins-rhai/src/lib.rs
```

Note where `.rhai` files are discovered (likely `walkdir` over a `plugins/` directory) and how they're parsed. The loader will keep doing discovery but stop reading file contents directly.

- [ ] **Step 2: Add a Salsa input for plugin source**

In `crates/stratum-core/src/db.rs` (or wherever Salsa inputs live — find it via `grep -rn '#\[salsa::input\]' crates/`), add:
```rust
#[salsa::input]
pub struct PluginSource {
    pub path: camino::Utf8PathBuf,
    pub contents: String,
}

#[salsa::tracked]
pub fn compiled_plugin(db: &dyn StratumDb, src: PluginSource) -> Result<CompiledPlugin, PluginCompileError> {
    let contents = src.contents(db);
    crate::compile_rhai(&contents).map_err(Into::into)
}
```

The exact module path for `compile_rhai` depends on whether plugin compilation moves to core or stays in `stratum-plugins-rhai`. Recommended: keep compile logic in `stratum-plugins-rhai` and expose it as a free function `stratum_plugins_rhai::compile(contents: &str) -> Result<rhai::AST, ...>`. The Salsa `#[tracked]` query lives in whichever crate owns the database.

- [ ] **Step 3: Update the loader to populate the input from disk**

In `crates/stratum-plugins-rhai/src/loader.rs`, add a `load_plugins_into_db` function that:
1. Discovers `.rhai` files under the configured plugin directory.
2. For each, reads the file once, calls `db.create_plugin_source(path, contents)`.
3. Returns the list of `PluginSource` handles.

Skeleton:
```rust
pub fn load_plugins_into_db(
    db: &mut dyn StratumDb,
    plugins_dir: &camino::Utf8Path,
) -> std::io::Result<Vec<PluginSource>> {
    let mut sources = Vec::new();
    for entry in walkdir::WalkDir::new(plugins_dir) {
        let entry = entry?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("rhai") {
            continue;
        }
        let path = camino::Utf8PathBuf::from_path_buf(entry.path().to_owned())
            .map_err(|p| std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("non-utf8 plugin path: {}", p.display())))?;
        let contents = std::fs::read_to_string(&path)?;
        sources.push(PluginSource::new(db, path, contents));
    }
    Ok(sources)
}
```

- [ ] **Step 4: Add an invalidation helper**

```rust
pub fn invalidate_plugin(db: &mut dyn StratumDb, src: PluginSource, new_contents: String) {
    src.set_contents(db).to(new_contents);
}
```

`set_contents` is Salsa's generated setter. Calling it bumps the input revision and invalidates every tracked query that read `contents(db)`.

- [ ] **Step 5: Add unit tests**

`crates/stratum-plugins-rhai/tests/loader_salsa.rs`:
```rust
//! Verify that mutating a PluginSource input causes `compiled_plugin` to re-run.

#[test]
fn changing_plugin_source_invalidates_compiled_plugin() {
    let mut db = TestDb::default();
    let src = PluginSource::new(&mut db, "/p/a.rhai".into(), "fn bad() { undefined }".into());
    let first = compiled_plugin(&db, src);
    assert!(first.is_err(), "expected compile error for undefined ref");

    src.set_contents(&mut db).to("fn good() { 1 }".into());
    let second = compiled_plugin(&db, src);
    assert!(second.is_ok(), "expected success after fix");
}
```

Where `TestDb` is the lightweight Salsa database harness already used elsewhere in the project (find it via `grep -rn '#\[salsa::db\]' crates/`).

- [ ] **Step 6: Run tests**

```bash
cargo test -p stratum-plugins-rhai
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/stratum-core crates/stratum-plugins-rhai
git commit -m "feat(plugins): plugin source as Salsa input"
```

---

### Task 2: File watcher with debounced re-read

**Files:**
- Create: `crates/stratum-plugins-rhai/src/watcher.rs`
- Modify: `crates/stratum-plugins-rhai/Cargo.toml` (add `notify`, `notify-debouncer-mini`)
- Modify: `crates/stratum-plugins-rhai/src/lib.rs` (re-export)

We need OS file-change events for the plugin directory, debounced (filesystems can fire multiple events per save), routed back to the host to mutate the Salsa input.

- [ ] **Step 1: Add deps**

In `crates/stratum-plugins-rhai/Cargo.toml`, append to `[dependencies]`:
```toml
notify = "6"
notify-debouncer-mini = "0.4"
```

- [ ] **Step 2: Write the failing test**

`crates/stratum-plugins-rhai/tests/watcher.rs`:
```rust
//! Watcher emits a single event per save (after debounce).

use std::time::Duration;
use std::sync::mpsc;
use tempfile::tempdir;

#[test]
fn watcher_emits_event_when_rhai_file_modified() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("rule.rhai");
    std::fs::write(&file, "fn check() {}").unwrap();

    let (tx, rx) = mpsc::channel();
    let _w = stratum_plugins_rhai::watch_plugins(dir.path(), Duration::from_millis(50), tx)
        .expect("watcher started");

    std::thread::sleep(Duration::from_millis(100)); // settle
    std::fs::write(&file, "fn check() { 1 }").unwrap();

    let event = rx.recv_timeout(Duration::from_secs(2)).expect("event arrives");
    assert_eq!(event.path, camino::Utf8PathBuf::from_path_buf(file).unwrap());
}

#[test]
fn watcher_ignores_non_rhai_files() {
    let dir = tempdir().unwrap();
    let (tx, rx) = mpsc::channel();
    let _w = stratum_plugins_rhai::watch_plugins(dir.path(), Duration::from_millis(50), tx)
        .expect("watcher started");

    std::fs::write(dir.path().join("ignored.txt"), "hi").unwrap();
    assert!(rx.recv_timeout(std::time::Duration::from_millis(300)).is_err());
}
```

- [ ] **Step 3: Run, verify it fails (function not exported)**

```bash
cargo test -p stratum-plugins-rhai watcher
```

Expected: FAIL with "cannot find function `watch_plugins`".

- [ ] **Step 4: Implement the watcher**

`crates/stratum-plugins-rhai/src/watcher.rs`:
```rust
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};

#[derive(Debug, Clone)]
pub struct PluginChange {
    pub path: camino::Utf8PathBuf,
}

pub fn watch_plugins(
    dir: &Path,
    debounce: Duration,
    tx: Sender<PluginChange>,
) -> notify::Result<Debouncer<notify::RecommendedWatcher>> {
    let dir_owned = dir.to_owned();
    let mut debouncer = new_debouncer(debounce, move |res: notify_debouncer_mini::DebounceEventResult| {
        let events = match res {
            Ok(events) => events,
            Err(_) => return,
        };
        for event in events {
            if event.kind != DebouncedEventKind::Any { continue; }
            let Some(ext) = event.path.extension() else { continue };
            if ext != "rhai" { continue; }
            let Ok(utf8) = camino::Utf8PathBuf::from_path_buf(event.path.clone()) else { continue };
            let _ = tx.send(PluginChange { path: utf8 });
            let _ = &dir_owned;
        }
    })?;
    debouncer.watcher().watch(dir, RecursiveMode::Recursive)?;
    Ok(debouncer)
}
```

Re-export in `lib.rs`:
```rust
pub mod watcher;
pub use watcher::{watch_plugins, PluginChange};
```

- [ ] **Step 5: Run, verify it passes**

```bash
cargo test -p stratum-plugins-rhai watcher
```

Expected: PASS, 2 tests.

- [ ] **Step 6: Commit**

```bash
git add crates/stratum-plugins-rhai
git commit -m "feat(plugins): debounced filesystem watcher for .rhai files"
```

---

### Task 3: Wire watcher into the LSP server

**Files:**
- Modify: `crates/stratum-lsp/src/server.rs` (or wherever the LSP backend struct lives)
- Modify: `crates/stratum-lsp/src/state.rs` (track watcher handle in shared state)

In LSP, the watcher runs from the moment the server initializes until it shuts down. When a `PluginChange` arrives, the server mutates the Salsa input for that plugin and triggers a re-publish of diagnostics for all open documents.

- [ ] **Step 1: Read the existing LSP backend struct**

```bash
cat crates/stratum-lsp/src/server.rs
cat crates/stratum-lsp/src/state.rs
```

Note where the Salsa database is stored (probably `Arc<Mutex<StratumDb>>` inside the backend).

- [ ] **Step 2: Spawn the watcher in `initialized`**

In `server.rs`, find the `LanguageServer::initialized` handler (or whichever method runs after `initialize` completes). Spawn the watcher there:
```rust
async fn initialized(&self, _params: InitializedParams) {
    let plugins_dir = self.state.lock().await.plugins_dir().to_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    let debouncer = match stratum_plugins_rhai::watch_plugins(
        plugins_dir.as_std_path(),
        std::time::Duration::from_millis(50),
        tx,
    ) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("plugin watcher failed to start: {e}");
            return;
        }
    };
    self.state.lock().await.set_plugin_watcher(debouncer);

    let backend = self.clone();
    tokio::task::spawn_blocking(move || {
        while let Ok(change) = rx.recv() {
            // Hop back onto the async runtime to update Salsa and re-publish.
            let backend = backend.clone();
            tokio::runtime::Handle::current().spawn(async move {
                if let Err(e) = backend.reload_plugin(&change.path).await {
                    tracing::warn!("plugin reload failed: {e}");
                }
            });
        }
    });
}
```

- [ ] **Step 3: Implement `reload_plugin`**

In the same file, add:
```rust
async fn reload_plugin(&self, path: &camino::Utf8Path) -> Result<(), std::io::Error> {
    let contents = std::fs::read_to_string(path)?;
    let mut state = self.state.lock().await;
    state.update_plugin_source(path, contents);
    let open_docs = state.open_documents().clone();
    drop(state);

    for uri in open_docs {
        if let Err(e) = self.recompute_diagnostics(&uri).await {
            tracing::warn!("re-publish for {uri} failed: {e}");
        }
    }
    tracing::info!("plugin reloaded: {path}");
    Ok(())
}
```

`update_plugin_source` on `State` looks up the existing `PluginSource` (cached by path) and calls `set_contents().to(...)`. If the plugin is new (didn't exist before), it creates a fresh `PluginSource`.

- [ ] **Step 4: Add a manual smoke test (documented, not automated yet)**

In `crates/stratum-lsp/README.md` (create if missing), add a "Manual hot-reload check" section:
```markdown
## Manual hot-reload check

1. `cargo run -p stratum-lsp` from a project that has `.rhai` plugins.
2. Connect any LSP client (e.g. `editor-extensions/vscode`).
3. Open a file that the plugin flags. Confirm a diagnostic appears.
4. Edit the plugin file; save it.
5. Within ~100 ms, the diagnostic should disappear (or change) without restarting the server.
```

- [ ] **Step 5: Commit**

```bash
git add crates/stratum-lsp
git commit -m "feat(lsp): hot-reload .rhai plugins via Salsa input invalidation"
```

---

### Task 4: Wire watcher into the CLI watch mode

**Files:**
- Modify: `crates/stratum-lint/src/commands/lint.rs` (or wherever `--watch` lives)

The CLI already has a watch mode (per Phase 4 memory). Plumb the same `watch_plugins` into it so editing `.rhai` files while `stratum-lint --watch` is running re-runs lint.

- [ ] **Step 1: Read the current watch impl**

```bash
grep -n 'watch' crates/stratum-lint/src/commands/lint.rs
```

Find where it spawns a file watcher for source files. Add a parallel watcher for plugins next to it.

- [ ] **Step 2: Add the plugin watcher**

In the same watch loop, alongside the source watcher:
```rust
let (plugin_tx, plugin_rx) = std::sync::mpsc::channel();
let _plugin_watcher = stratum_plugins_rhai::watch_plugins(
    plugins_dir.as_std_path(),
    std::time::Duration::from_millis(50),
    plugin_tx,
)?;

// Inside the select! or merge loop, handle plugin_rx:
while let Ok(change) = plugin_rx.try_recv() {
    let contents = std::fs::read_to_string(&change.path)?;
    db.update_plugin_source(&change.path, contents);
    rerun_lint = true;
}
```

- [ ] **Step 3: Manual smoke test**

```bash
cargo run -p stratum-lint -- lint tests/fixtures/exemplar-stratum-app --watch
# In another terminal, touch a .rhai plugin under tests/fixtures/exemplar-stratum-app/plugins/
echo '' >> tests/fixtures/exemplar-stratum-app/plugins/<some>.rhai
```

Expected: the watch-mode log shows a re-lint within ~100 ms.

- [ ] **Step 4: Commit**

```bash
git add crates/stratum-lint
git commit -m "feat(lint): hot-reload .rhai plugins in watch mode"
```

---

### Task 5: Perf gate — reload-latency benchmark

**Files:**
- Create: `crates/stratum-plugins-rhai/benches/hot_reload.rs`
- Modify: `crates/stratum-plugins-rhai/Cargo.toml`

Salsa-based reload should be cheap. Lock in <50 ms p95.

- [ ] **Step 1: Add criterion bench**

In `crates/stratum-plugins-rhai/Cargo.toml`:
```toml
[dev-dependencies]
criterion = { workspace = true }
tempfile = "3"

[[bench]]
name = "hot_reload"
harness = false
```

- [ ] **Step 2: Write the bench**

`crates/stratum-plugins-rhai/benches/hot_reload.rs`:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn bench_reload(c: &mut Criterion) {
    let mut db = TestDb::default(); // same harness as the loader test
    let src = stratum_plugins_rhai::PluginSource::new(
        &mut db,
        "/p/a.rhai".into(),
        "fn check(node) { node.kind == \"Module\" }".into(),
    );
    let _ = stratum_plugins_rhai::compiled_plugin(&db, src);

    let mut counter = 0u64;
    c.bench_function("reload_one_plugin", |b| {
        b.iter(|| {
            counter += 1;
            src.set_contents(&mut db).to(format!("fn check(node) {{ node.id == \"{counter}\" }}"));
            black_box(stratum_plugins_rhai::compiled_plugin(&db, src));
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(50).measurement_time(Duration::from_secs(5));
    targets = bench_reload
}
criterion_main!(benches);
```

- [ ] **Step 3: Run the bench**

```bash
cargo bench -p stratum-plugins-rhai --bench hot_reload
```

Expected: prints reload time, ideally <5 ms per iteration (Salsa + Rhai parse of one small script). If >50 ms, investigate before declaring victory.

- [ ] **Step 4: Wire into CI bench workflow**

Look at `.github/workflows/bench.yml`. Add a step that runs this bench and fails if p95 > 50 ms:
```yaml
      - name: hot-reload bench
        run: |
          cargo bench -p stratum-plugins-rhai --bench hot_reload -- --output-format bencher | tee bench-out.txt
          # crude assertion: extract median-ish line and parse — refine if needed
          grep 'reload_one_plugin' bench-out.txt
```

A proper threshold assertion can be added later; for now we just record the number in CI output.

- [ ] **Step 5: Commit**

```bash
git add crates/stratum-plugins-rhai/Cargo.toml crates/stratum-plugins-rhai/benches .github/workflows/bench.yml
git commit -m "test(plugins): hot-reload latency benchmark"
```

---

### Task 6: Document the feature

**Files:**
- Modify: `docs/language/glossary.md` (add `Plugin hot-reload`)
- Modify: `editor-extensions/vscode/README.md` and the Zed/WebStorm READMEs (mention the feature)

- [ ] **Step 1: Glossary entry**

Append to `docs/language/glossary.md`:
```markdown
### Plugin hot-reload

When a `.rhai` plugin file under the configured plugins directory is modified, Stratum re-compiles the script and re-runs diagnostics within ~50 ms — without restarting the LSP server or the `stratum-lint --watch` process. Implemented via a `notify`-backed file watcher that mutates a Salsa `PluginSource` input; downstream queries (compiled AST, per-file diagnostics) invalidate automatically. Plugin authors get an edit-save-see-result loop comparable to ESLint plugin development.

Limitations:
- Removing a `.rhai` file does not yet drop the corresponding plugin from the active set; it stays compiled until restart. Filed as follow-up.
- New `.rhai` files added after server start are picked up only on the next save (not on creation).
```

- [ ] **Step 2: Editor READMEs**

In each `editor-extensions/*/README.md`, add under the "How it works" or troubleshooting section:
```markdown
### Hot-reloading custom rules

Stratum watches the plugins directory of the current workspace. Save a `.rhai` file and diagnostics refresh within ~50 ms — no restart needed.
```

- [ ] **Step 3: Commit**

```bash
git add docs/language editor-extensions
git commit -m "docs: plugin hot-reload"
```

---

## Done definition

- `.rhai` plugin source lives behind a Salsa input; changing it invalidates downstream queries
- File watcher emits debounced change events for `.rhai` files
- LSP server reloads on save without restart; bench confirms <50 ms reload latency
- CLI `--watch` mode reloads plugins on save
- Hot-reload bench runs in CI
- Glossary + editor READMEs mention the feature

## Out of scope

- Reload on plugin deletion / creation (only modification). Filed as follow-up.
- Network-mounted plugin directories. `notify` works on local filesystems; users on NFS/SMB may need polling.
- Plugin sandboxing changes. The Rhai sandbox stays as-is.
