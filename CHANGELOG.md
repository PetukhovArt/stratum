# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The **Graph Snapshot** version is the on-wire contract between `stratum-lint`,
`stratum-lsp`, and the Visualizer. It is tracked separately from the package
version; a bump requires a coordinated frontend release.

## Unreleased

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
