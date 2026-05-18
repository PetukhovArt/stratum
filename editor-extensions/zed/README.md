# Stratum for Zed

Architectural linting in [Zed](https://zed.dev) via Stratum's LSP server.

## Install (local dev)

1. Install Stratum and make sure `stratum-lsp` is on your `PATH`:

   ```bash
   cargo install --path ../../crates/stratum-lsp
   stratum-lsp --version
   ```

   Or grab the prebuilt binaries from the
   [GitHub release](https://github.com/PetukhovArt/stratum/releases) and add
   them to your `PATH`.

2. Install this extension in Zed:
   - Open Zed
   - `cmd-shift-X` (macOS) / `ctrl-shift-X` (Linux) — "Install Dev Extension"
   - Select this directory (`editor-extensions/zed/`).

3. Open a TypeScript / Vue project containing `stratum.toml`. Diagnostics appear
   as red/yellow squiggles like any other LSP-backed linter.

## Seeing Stratum's logs

Stratum forwards every `tracing` event to the LSP `window/logMessage` channel,
so Zed shows them in:

- `cmd-shift-P` → `zed: open log`, or
- `View → Toggle Right Dock → LSP Logs`.

Filter for `stratum-lsp` to see startup, rule timings, and errors.

## Troubleshooting

- **"stratum-lsp not found"** — Zed inherits the PATH of the shell it was
  launched from. Either move `stratum-lsp` to a system-wide location or launch
  Zed from a terminal where `which stratum-lsp` works.
- **No diagnostics** — confirm the workspace has a `stratum.toml`, or run
  `stratum-lint init` to generate one. Stratum still runs with inferred
  defaults, but the LSP log will tell you what it inferred.

## Building the WASM (optional, for marketplace publishing)

```bash
rustup target add wasm32-wasi
cargo build --release --target wasm32-wasi
```

The resulting `target/wasm32-wasi/release/stratum_zed_extension.wasm` is what
Zed loads. Local dev installs (above) build it for you.
