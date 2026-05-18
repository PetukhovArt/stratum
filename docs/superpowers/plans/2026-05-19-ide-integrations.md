# IDE Integrations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Stratum trivially usable from Zed and WebStorm — install Stratum binaries, drop in a per-IDE config, and see live diagnostics with errors surfaced in each IDE's developer console / log panel.

**Architecture:** The existing `stratum-lsp` binary (already a separate cargo bin, already in releases via cargo-dist) is the integration point. Each IDE gets a minimal adapter: Zed gets a native extension under `editor-extensions/zed/`; WebStorm gets an LSP4IJ configuration under `editor-extensions/webstorm/`. Tower-lsp's logging is routed to LSP `window/logMessage` so users see Stratum's logs in the IDE's "LSP" panel without extra setup.

**Tech Stack:** Rust (Zed extension uses Rust glue), TOML (Zed `extension.toml`), JSON (LSP4IJ config), `tracing` + `tracing-subscriber` for log routing inside `stratum-lsp`.

**Depends on:** None hard, but works best after web-client integration (real diagnostics make the demo screenshots compelling).

---

### Task 1: Route `tracing` logs to LSP `window/logMessage`

**Files:**
- Modify: `crates/stratum-lsp/src/main.rs`
- Modify: `crates/stratum-lsp/Cargo.toml` (add `tracing`, `tracing-subscriber`)
- Create: `crates/stratum-lsp/src/log_layer.rs`

Right now LSP server logs likely go to stderr or are silent. We want them visible in the IDE's LSP-log panel via `window/logMessage` LSP notifications, so users debugging "why isn't Stratum running?" don't need to hunt for log files.

- [ ] **Step 1: Add tracing dependencies**

Edit `crates/stratum-lsp/Cargo.toml`, append to `[dependencies]`:
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

Run `cargo build -p stratum-lsp` to confirm it compiles.

- [ ] **Step 2: Write a failing test for the log layer**

`crates/stratum-lsp/src/log_layer.rs`:
```rust
//! Tracing layer that forwards events to an LSP `window/logMessage` channel.

use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tower_lsp::lsp_types::{MessageType, LogMessageParams};
use tracing::{Event, Subscriber};
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
        let level = *event.metadata().level();
        let msg_type = match level {
            tracing::Level::ERROR => MessageType::ERROR,
            tracing::Level::WARN => MessageType::WARNING,
            tracing::Level::INFO => MessageType::INFO,
            _ => MessageType::LOG,
        };
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        let _ = self.tx.send(LogMessageParams { typ: msg_type, message: visitor.0 });
    }
}

struct MessageVisitor(String);
impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::unbounded_channel;
    use tracing_subscriber::prelude::*;

    #[tokio::test]
    async fn forwards_info_event_as_info_log_message() {
        let (tx, mut rx) = unbounded_channel();
        let layer = LspLogLayer::new(tx);
        let _guard = tracing_subscriber::registry().with(layer).set_default();
        tracing::info!("hello-stratum");
        let msg = rx.recv().await.expect("log message expected");
        assert_eq!(msg.typ, MessageType::INFO);
        assert!(msg.message.contains("hello-stratum"));
    }
}
```

- [ ] **Step 3: Run the test, verify it fails initially (file doesn't exist), then passes after creating it**

```bash
cargo test -p stratum-lsp log_layer
```

Expected: PASS once the file compiles.

- [ ] **Step 4: Wire it into the server**

Read `crates/stratum-lsp/src/main.rs` first to see its current structure. Then modify so that during initialization the server creates an `unbounded_channel`, installs `LspLogLayer` as the global tracing subscriber, and spawns a task that drains the channel and forwards each `LogMessageParams` to the connected LSP client (`client.log_message(typ, message).await`).

The skeleton patch (placement depends on current `main.rs` shape):
```rust
use crate::log_layer::LspLogLayer;
use tokio::sync::mpsc::unbounded_channel;
use tracing_subscriber::prelude::*;

// Inside or before tower_lsp::Server::new(...):
let (log_tx, mut log_rx) = unbounded_channel();
let subscriber = tracing_subscriber::registry()
    .with(LspLogLayer::new(log_tx))
    .with(tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,stratum=debug")));
subscriber.init();

// After the server starts, attach to its client:
let client = backend.client.clone();
tokio::spawn(async move {
    while let Some(params) = log_rx.recv().await {
        client.log_message(params.typ, params.message).await;
    }
});
```

Declare `mod log_layer;` in `main.rs`.

- [ ] **Step 5: Add a smoke test of an end-to-end log delivery**

In `crates/stratum-lsp/tests/`, add `log_forwarding.rs`:
```rust
//! Spin up the LSP server in-process, send `initialize`, verify a `window/logMessage` arrives.

#[tokio::test]
async fn server_emits_window_log_message_on_init() {
    // Test scaffolding here depends on the existing test helpers in this crate;
    // pattern after any existing tower-lsp integration test (e.g. `diagnostics.rs`).
    // Assertion: at least one window/logMessage with MessageType::INFO is received
    // within 500ms of `initialized` notification.
}
```

If no existing helper makes this easy, file as a follow-up and ship without the integration test — but DON'T skip Steps 1-4.

- [ ] **Step 6: Commit**

```bash
git add crates/stratum-lsp
git commit -m "feat(lsp): forward tracing events to LSP window/logMessage"
```

---

### Task 2: Zed extension

**Files:**
- Create: `editor-extensions/zed/extension.toml`
- Create: `editor-extensions/zed/Cargo.toml`
- Create: `editor-extensions/zed/src/lib.rs`
- Create: `editor-extensions/zed/README.md`

Zed extensions are Rust WASM modules registered via `extension.toml`. For an LSP-only extension (no custom UI), the Rust glue is ~20 lines.

- [ ] **Step 1: Create `extension.toml`**

`editor-extensions/zed/extension.toml`:
```toml
id = "stratum"
name = "Stratum"
description = "Architectural linter for TypeScript / Vue projects"
version = "0.1.0"
schema_version = 1
authors = ["Stratum contributors"]
repository = "https://github.com/PetukhovArt/stratum"

[language_servers.stratum-lsp]
name = "stratum-lsp"
languages = ["TypeScript", "TSX", "Vue"]
```

- [ ] **Step 2: Create `Cargo.toml` for the WASM glue**

`editor-extensions/zed/Cargo.toml`:
```toml
[package]
name = "stratum-zed-extension"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib"]
path = "src/lib.rs"

[dependencies]
zed_extension_api = "0.0.7"
```

This extension is NOT a member of the main cargo workspace — it compiles to `wasm32-wasi` separately. Add an exclusion in the workspace `Cargo.toml`:
```toml
[workspace]
exclude = ["editor-extensions/zed"]
```

- [ ] **Step 3: Create the WASM glue**

`editor-extensions/zed/src/lib.rs`:
```rust
use zed_extension_api as zed;

struct StratumExtension;

impl zed::Extension for StratumExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: "stratum-lsp".into(),
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(StratumExtension);
```

This assumes `stratum-lsp` is on the user's `PATH`. The README in Step 4 documents this.

- [ ] **Step 4: Build the extension WASM**

```bash
cd editor-extensions/zed
rustup target add wasm32-wasi
cargo build --release --target wasm32-wasi
```

Expected: `target/wasm32-wasi/release/stratum_zed_extension.wasm` exists.

- [ ] **Step 5: Write the README**

`editor-extensions/zed/README.md`:
```markdown
# Stratum for Zed

Architectural linting in Zed via LSP.

## Install (local dev)

1. Install Stratum:
   ```
   cargo install stratum-lint stratum-lsp
   # OR via the release artifacts: https://github.com/PetukhovArt/stratum/releases
   ```
   Ensure `stratum-lsp` is on your `PATH`. Verify with `stratum-lsp --version`.

2. Install this extension in Zed:
   - Open Zed
   - `cmd-shift-X` (macOS) / `ctrl-shift-X` (Linux) → "Install Dev Extension"
   - Select the `editor-extensions/zed` directory in this repo

3. Open a TypeScript / Vue project that has `stratum.toml`. Diagnostics appear as red/yellow squiggles.

## See what Stratum is doing

Zed surfaces LSP logs in:
- `cmd-shift-P` → "zed: open log"
- Or: `View → Toggle Right Dock → LSP Logs`

You'll see entries from `stratum-lsp` — startup messages, rule timings, errors.

## Troubleshooting

- "stratum-lsp not found": verify `stratum-lsp` is on PATH for the shell Zed was launched from.
- No diagnostics: confirm the workspace contains `stratum.toml`. Without it, Stratum loads defaults but you should still see lint output — check the LSP log panel.
```

- [ ] **Step 6: Commit**

```bash
git add editor-extensions/zed Cargo.toml
git commit -m "feat(editor): zed extension"
```

---

### Task 3: WebStorm via LSP4IJ

**Files:**
- Create: `editor-extensions/webstorm/lsp4ij-stratum.json`
- Create: `editor-extensions/webstorm/README.md`

LSP4IJ is JetBrains' official-ish open-source plugin that adapts any LSP server to IntelliJ-family IDEs (WebStorm, IDEA, PyCharm, etc.). It uses a JSON spec to describe how to launch + match languages. Once installed and imported, the user gets diagnostics inline.

- [ ] **Step 1: Create the LSP4IJ JSON descriptor**

`editor-extensions/webstorm/lsp4ij-stratum.json`:
```json
{
  "name": "Stratum",
  "id": "stratum",
  "description": "Architectural linter for TypeScript / Vue projects",
  "languageMappings": [
    { "language": "TypeScript", "languageId": "typescript" },
    { "language": "Vue.js", "languageId": "vue" }
  ],
  "fileTypeMappings": [
    { "fileNamePattern": "*.ts", "languageId": "typescript" },
    { "fileNamePattern": "*.tsx", "languageId": "typescriptreact" },
    { "fileNamePattern": "*.vue", "languageId": "vue" }
  ],
  "serverDefinitions": [
    {
      "commandLine": "stratum-lsp",
      "args": [],
      "environment": {},
      "rootPathMatcher": "stratum.toml"
    }
  ],
  "trace": "messages"
}
```

The `"trace": "messages"` flag makes LSP4IJ log every message — visible in `View → Tool Windows → LSP Console`. That's the "dev console errors" the user asked for.

- [ ] **Step 2: Write the README**

`editor-extensions/webstorm/README.md`:
```markdown
# Stratum for WebStorm (and other JetBrains IDEs)

Architectural linting in WebStorm / IntelliJ IDEA / etc. via LSP4IJ.

## Install

1. Install Stratum:
   ```
   cargo install stratum-lint stratum-lsp
   # OR via the release artifacts: https://github.com/PetukhovArt/stratum/releases
   ```
   Verify `stratum-lsp --version` works from your shell.

2. Install [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) in WebStorm:
   - `File → Settings → Plugins → Marketplace`
   - Search "LSP4IJ", install, restart.

3. Import this descriptor:
   - `File → Settings → Languages & Frameworks → Language Servers → +`
   - Select "Import from JSON"
   - Choose `editor-extensions/webstorm/lsp4ij-stratum.json` from this repo

4. Open a TypeScript / Vue project containing `stratum.toml`. Squiggles appear inline.

## See what Stratum is doing (dev console)

- `View → Tool Windows → LSP Console`
- Filter on "Stratum"
- You'll see `window/logMessage` entries from `stratum-lsp`: startup, rule timings, errors.

The descriptor has `"trace": "messages"` set, so the console shows every LSP request/response.

## Troubleshooting

- "stratum-lsp not found": WebStorm uses the GUI shell's PATH, which may differ from your terminal's. Either:
  - Move `stratum-lsp` to `/usr/local/bin` (Unix) / `C:\Windows\System32` (Windows), or
  - Set the absolute path in the descriptor's `commandLine`.
- LSP4IJ shows "server failed": click the server name in the LSP Console to see the underlying error.
```

- [ ] **Step 3: Validate the JSON**

```bash
node -e "JSON.parse(require('fs').readFileSync('editor-extensions/webstorm/lsp4ij-stratum.json', 'utf-8'))" && echo OK
```

Expected: `OK`.

- [ ] **Step 4: Commit**

```bash
git add editor-extensions/webstorm
git commit -m "feat(editor): WebStorm support via LSP4IJ"
```

---

### Task 4: Test Zed integration manually + screenshot

**Files:**
- Create: `editor-extensions/zed/docs/screenshot-zed.png`

- [ ] **Step 1: Install Zed if not already present**

Skip if Zed is already installed. Otherwise: `brew install zed` (macOS) or download from https://zed.dev.

- [ ] **Step 2: Install Stratum binaries to PATH**

```bash
cargo install --path crates/stratum-lint
cargo install --path crates/stratum-lsp
which stratum-lsp
```

Expected: a path is printed.

- [ ] **Step 3: Install the Zed dev extension**

In Zed: cmd-shift-X (or ctrl-shift-X) → "Install Dev Extension" → select `editor-extensions/zed`.

- [ ] **Step 4: Open `D:/web-projects/web-client` in Zed**

Confirm:
- Diagnostics appear on at least one file (assuming web-client violates at least one rule — Plan 2 should have ensured this)
- LSP log panel (`cmd-shift-P → zed: open log` or right-dock LSP Logs) shows Stratum output

- [ ] **Step 5: Screenshot diagnostics + log panel**

Save a screenshot under `editor-extensions/zed/docs/screenshot-zed.png`. This is for the README and proof-of-life.

- [ ] **Step 6: Commit**

```bash
git add editor-extensions/zed/docs/screenshot-zed.png
git commit -m "docs(editor): zed install screenshot"
```

---

### Task 5: Test WebStorm integration manually + screenshot

**Files:**
- Create: `editor-extensions/webstorm/docs/screenshot-webstorm.png`

Same pattern as Task 4 but for WebStorm + LSP4IJ.

- [ ] **Step 1: Install WebStorm + LSP4IJ (manual)**

JetBrains Toolbox is the easiest install path. Install LSP4IJ from the marketplace inside WebStorm.

- [ ] **Step 2: Import the Stratum descriptor**

`File → Settings → Languages & Frameworks → Language Servers → + → Import from JSON → editor-extensions/webstorm/lsp4ij-stratum.json`.

- [ ] **Step 3: Open web-client; confirm diagnostics + LSP console output**

- [ ] **Step 4: Screenshot**

Save under `editor-extensions/webstorm/docs/screenshot-webstorm.png`.

- [ ] **Step 5: Commit**

```bash
git add editor-extensions/webstorm/docs/screenshot-webstorm.png
git commit -m "docs(editor): webstorm install screenshot"
```

---

### Task 6: Cross-link from main README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add an "Editor support" section to the README**

Open `README.md`, find the existing section structure. Add (after the install section, before contributing):
```markdown
## Editor support

Stratum exposes itself as an LSP server (`stratum-lsp`), so any editor with an LSP client can use it. We ship configs for:

| Editor | Setup | Docs |
|--------|-------|------|
| VS Code | Install the extension in `editor-extensions/vscode/` | [README](editor-extensions/vscode/README.md) |
| Zed | Install the dev extension in `editor-extensions/zed/` | [README](editor-extensions/zed/README.md) |
| WebStorm / JetBrains | Import the LSP4IJ descriptor in `editor-extensions/webstorm/` | [README](editor-extensions/webstorm/README.md) |
| Other LSP clients | Run `stratum-lsp` as your LSP server for TS / Vue files | — |

Logs from Stratum surface in each editor's LSP log panel (search for `Stratum` entries).
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs(readme): cross-link Zed and WebStorm support"
```

---

### Task 7: Add Zed extension WASM to release artifacts

**Files:**
- Modify: `.github/workflows/release.yml`

Optional but useful: bundle the Zed extension WASM into the GitHub release so users can install without compiling.

- [ ] **Step 1: Add a build step for the WASM**

In `.github/workflows/release.yml`, in the `build` job (after rust-toolchain setup), add:
```yaml
      - name: build zed extension wasm
        if: matrix.os == 'ubuntu-latest' && matrix.target == 'x86_64-unknown-linux-gnu'
        run: |
          rustup target add wasm32-wasi
          cd editor-extensions/zed
          cargo build --release --target wasm32-wasi
          mkdir -p ../../target/distrib/zed
          cp target/wasm32-wasi/release/stratum_zed_extension.wasm ../../target/distrib/zed/
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): bundle zed extension wasm into release artifacts"
```

---

## Done definition

- `stratum-lsp` forwards tracing events to LSP `window/logMessage`
- `editor-extensions/zed/` builds and installs as a Zed dev extension
- `editor-extensions/webstorm/lsp4ij-stratum.json` imports cleanly into LSP4IJ
- Both READMEs are step-by-step with troubleshooting + screenshot
- Main `README.md` cross-links to all three editor configs
- Zed extension WASM is built and uploaded with each release

## Out of scope

- Marketplace publishing (Zed marketplace, JetBrains plugin marketplace). Manual install is fine for v0.1.
- Custom UI surfaces in each IDE (quick-fixes, code actions beyond what LSP provides). LSP gets us most of the value.
- Native JetBrains plugin (no LSP4IJ dependency). Considered overkill until adoption demands it.
