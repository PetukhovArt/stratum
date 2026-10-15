# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The **Graph Snapshot** version is the on-wire contract between `stratum-lint`,
`stratum-lsp`, and the Visualizer. It is tracked separately from the package
version; a bump requires a coordinated frontend release.

## Unreleased

### Removed

- Editor extensions (`editor-extensions/` for VS Code, Zed, WebStorm). `stratum-lsp` stays and targets AI harnesses; see `docs/harness.md`.

### Changed

- **Breaking:** the purity rank is called purity everywhere. `// @stratum-stage N` is now `// @stratum-purity N`, and the old annotation is no longer recognised. The rule `stratum/stage-purity` is now `stratum/purity`, `Module.stage` is `Module.purity`, `BuildConfig.default_stage` is `default_purity`, Rhai `m.stage` is `m.purity`, and `context` prints `purity`. Baseline fingerprints keyed by the old slug go stale. The Graph Snapshot keeps the field name `stage` until v2.
- **Breaking:** `project.root` is gone from the config (nothing read it; the graph always walks ROOT), and `project` itself is optional. Override entries take `severity` only: `options` or `script` there is a config error.
- A bad config fails every surface: `rules`, `explain`, `context`, `visualize` and `snapshot` exit 2 like `lint`, invalid `overrides[].files` globs are rejected at load, and `stratum-lsp` reports them via `window/showMessage` instead of publishing no diagnostics.
- `stratum-lsp` hides baselined violations and re-analyzes when a config file in the `extends` chain or `stratum.baseline.json` changes. The PostToolUse hook checks `.jsx/.mjs/.cjs` edits, and the LSP watches `.mts/.cts` files.
- `/api/violations` carries rule slugs instead of numeric ids, and ROOT-relative paths.
- `stratum-lsp` no longer depends on the CLI crate: both use the new `stratum-engine` crate.
- `stratum-lsp`: `initialize` returns at once and analysis runs in the background; reads `stratum.config.jsonc`; re-analyzes when the config or a plugin script changes; handles `didSave`/`didClose` and `file:///c%3A/...` URIs; diagnostics carry `source: "stratum"`, the rule slug as `code` (plugin rules included), the suggestion, and `relatedInformation`.

## 0.1.0 — 2026-05-18

First public release. Phases 0–10 of the implementation roadmap.

### Added

- `stratum-lint` CLI with `lint`, `init`, `snapshot`, `visualize`, `diff` subcommands.
- 9 baseline rules: `no-cross-layer-import`, `no-circular-deps`, `stage-purity`, `miller-limit`, `depth-ratio`, `visibility-scope`, `cross-entity-pattern`, `deep-module`, `promotion-pressure`.
- TS/JS/JSX/TSX extractor via OXC (`stratum-parser-ts`).
- Vue SFC MVP extractor (`stratum-parser-vue`) — script blocks only.
- LSP server (`stratum-lsp`) with hover + document links + debounced diagnostics.
- Visualizer transport: `stratum-lint visualize` serves the **Graph Snapshot** at `/api/snapshot` and bundles the frontend via `rust-embed`.
- Rhai plugin runtime (`stratum-plugins-rhai`) — custom rules in a sandboxed DSL.
- Three reporters: terminal, JSON, SARIF v2.1.
- `stratum.config.jsonc` with per-file overrides + JSON schema.
- `--watch` mode with 200 ms debounce.
- `cargo-dist`-based release pipeline.
- npm / Homebrew / Scoop distribution wrappers.

### Snapshot

- `GraphSnapshot.version = 1`.

### Known limitations

- Vue `<template>` and `<style>` blocks unparsed; CSS module imports not surfaced.
- Type-only edges treated identically to runtime edges.
- LSP `SourceLocation` is single-point (no end-column).
- Rhai `ModuleView` lacks raw import specifier strings.
- Hot-reload for `.rhai` scripts piggybacks on `--watch` rather than Salsa per-rule invalidation.
- Salsa per-rule tracked queries deferred; cold rebuilds are fast enough on the current fixtures.
- PixiJS visualizer rendering layer is scaffolded but not implemented; the transport works end-to-end.
- Empirical layout re-spike (`dagre-wasm` on host hardware) is filed as a follow-up.
