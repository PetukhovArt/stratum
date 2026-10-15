# Stratum

Compound DAG Architecture Methodology — a TypeScript/Vue architectural linter
with an LSP server and an interactive dependency visualizer.

## AI harness integration

Stratum is built to be the architecture check of an AI coding harness
(Claude Code, Cursor, Codex). Any stdio LSP client can launch `stratum-lsp`
for `.ts` / `.tsx` / `.vue` files and get per-edit diagnostics; the CLI covers
end-of-turn gates and on-demand context. No editor extensions are shipped.
See [docs/harness.md](docs/harness.md) for the surfaces and the CLI contract.
