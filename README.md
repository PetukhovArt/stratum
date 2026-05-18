# Stratum

Compound DAG Architecture Methodology — a TypeScript/Vue architectural linter
with an LSP server and an interactive dependency visualizer.

## Editor support

Stratum ships as a standalone LSP server (`stratum-lsp`), so any editor with an
LSP client can use it. We bundle ready-to-go configs for:

| Editor | Setup | Docs |
|--------|-------|------|
| VS Code | Install the extension under `editor-extensions/vscode/` | [README](editor-extensions/vscode/README.md) |
| Zed | Install the dev extension under `editor-extensions/zed/` | [README](editor-extensions/zed/README.md) |
| WebStorm / JetBrains | Import the LSP4IJ descriptor under `editor-extensions/webstorm/` | [README](editor-extensions/webstorm/README.md) |
| Other LSP clients | Point your client at `stratum-lsp` for TypeScript / Vue files | — |

Stratum forwards every `tracing` event to the LSP `window/logMessage` channel,
so its startup, rule timings, and errors all surface in your editor's LSP-log
panel — no separate log file to hunt down.
