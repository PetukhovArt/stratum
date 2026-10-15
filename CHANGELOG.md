# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The **Graph Snapshot** version is the on-wire contract between `stratum-lint`
(`snapshot`, `visualize`) and the Visualizer. It is tracked separately from the
package version; a bump requires a coordinated frontend release.

## Unreleased

### Breaking

- `--format json` is version 2: `{ "version": 2, "summary": {...}, "violations": [...] }`. Rules are named by slug (`stratum/no-circular-deps`, `custom/<name>`), never by numeric id, and paths are relative to ROOT.
- Graph Snapshot v2: `module.stage` is now `module.purity`; containers gain `path`, `kind` (`layer`, `slice`, `sub_unit`, `nested`), `stage` (the evolution stage, 1–4) and `entry`. The Visualizer accepts only v2.
- Stage is renamed to purity for the 1–4 rank: `// @stratum-stage N` is now `// @stratum-purity N` and the old annotation is no longer recognised; `stratum/stage-purity` is now `stratum/purity`; Rhai `m.stage` is `m.purity`; `context` prints `purity`. "Stage" now means only a container's evolution stage. Baseline entries keyed by the old slug go stale.
- `stratum/cross-entity-pattern` is replaced by `stratum/slice-fan-in`: distinct importing slices per slice, option `max_fan_in` (5, was `max_fanout`), off by default. No folder name (`entities`, `features`, `index.ts`, `_shared`) is hard-coded any more; slices come from `layers[].slices`.
- `project.root` is removed: nothing read it, the graph always walks ROOT. It is now an unknown key, and `project` is optional.
- Config validation is strict: an unknown key, an unknown rule slug, an option the rule does not take or a wrongly typed one, a `depends_on` naming an undeclared layer, a bad `project.ignore` or `overrides[].files` glob, an unknown preset or an `extends` cycle fails with exit 2, naming the slug and key. `lint`, `baseline`, `context`, `rules`, `explain`, `snapshot` and `visualize` exit 2 on it, as on any other tool failure; `hook` exits 1, Claude Code's non-blocking error.
- `overrides` set `severity` only; `options` or `script` in an override is a config error.
- Editor extensions are removed (`editor-extensions/` for VS Code, Zed, WebStorm). `stratum-lsp` stays and targets AI harnesses: see `docs/harness.md`.
- The `diff` subcommand is removed; it was a stub that always failed.

### Added

- `stratum-lint hook`: the Claude Code hook adapter. On `PostToolUse` of `Edit` / `Write` it lints the edited source file and returns its new error and warning violations as context; on `Stop` / `SubagentStop` it gates the changed files and blocks the stop on new errors.
- Gate options: `--format agent` (compact text for a model's context), `--max`, `--files`, `--changed[=REF]`, `--baseline` / `--no-baseline`. `stratum-lint baseline [--prune]` writes `stratum.baseline.json`, fingerprinted by slug, file and message with numbers masked.
- `stratum-lint context <file>`: where a file, existing or planned, sits and what it may import: layer, purity, container with kind, stage and entry, the enclosing slice, visible private zones, and a `may not import` line (sibling slices, sibling sub-units, files behind another container's entry).
- `stratum-lint explain <slug>` and `stratum-lint rules`, with built-in docs for every rule.
- Configurable structure: `layers[].slices` and a `structure` key (`entries`, `segments`, `private`). Containers (layer, slice, sub-unit, nested), their evolution stage and private zones are derived from the folder tree.
- Rules `stratum/public-api`, `stratum/sibling-isolation`, `stratum/restricted-calls` (with call tracking in the parser), `stratum/stage-mixing`, `stratum/container-breadth`, and the declarative `stratum/import-policy`.
- Config `extends`: a relative file or the presets `stratum:recommended` and `stratum:fsd`, merged by layer id and rule slug. `project.ignore` keeps files out of the graph, on top of built-in defaults. `stratum.schema.json`, referenced with `"$schema"`.
- Rhai plugins: `m.imports`, `fn check(m, options)`, an optional `fn doc()` that shows the rule in `rules` / `explain`; `print` and `debug` go to stderr.
- `.mts` / `.cts` sources. Export counting (`Exports`), including Vue `<script setup>` macros.
- Claude Code plugins `stratum` (hook + skill) and `stratum-lsp` (LSP), with a marketplace; `docs/agents-snippet.md` for Codex, Cursor and others. The npm wrapper ships `stratum-lsp` too.
- `stratum-lint visualize --config PATH` and `--no-open` (serve without launching the browser).
- Visualizer: rewritten in SolidJS + SVG with a fractal pack layout (PixiJS and dagre dropped), violations from `/api/violations`, an outline panel, a search palette, a path glob filter, a minimap, node and edge tooltips, a violators-only filter, cycle focus, colour modes layer / stage / purity, and stage and purity filters.

### Changed

- `stratum-lsp`:
  - `initialize` answers at once and the analysis runs in the background.
  - Unsaved buffers count: `didOpen` / `didChange` re-parse that document from its buffer and update the graph in memory, instead of a full rebuild per edit.
  - It reads `stratum.config.jsonc`, honours `stratum.baseline.json` (baselined violations are not published), and rebuilds when the config, a file in its `extends` chain, a plugin script, `tsconfig.json` or the baseline changes.
  - A config error is reported via `window/showMessage` instead of silently publishing nothing.
  - Diagnostics carry `source: "stratum"`, the rule slug as `code`, the suggestion and `relatedInformation`.
  - `file:///c%3A/...` URIs are accepted, and the process exits on `exit`.
  - The server no longer depends on the CLI crate: both use the new `stratum-engine` crate.
- `deep-module` and `depth-ratio` count real exports. `miller-limit` counts declared importers and `promotion-pressure` undeclared ones, so no importer counts twice. In a sliced layer these metrics measure across slices.
- `visibility-scope` fires: modules under a `structure.private` folder (default `_shared`) are private to the folder's parent. Before, every module was public.
- Zero-config applies the `stratum:recommended` severities, with `public-api`, `sibling-isolation` and `container-breadth` off until layers and structure are declared.
- The configured severity caps a rule's findings: a rule may report lower, never higher. `container-breadth` reports private-zone folders at `info` at most.
- `--files` resolves against the cwd, then ROOT, and a missing path exits 2. A scope matches every member of a violation, so editing any module of a cycle surfaces it.
- `--format agent` collapses info findings into one advisory line. Hook output never shows the `baseline --prune` hint.
- `/api/violations` carries rule slugs and ROOT-relative paths.

### Fixed

- Vue: a `<script>` tag end respects quoted attributes, so `generic="T extends X<Y>"` keeps the script body.

### Performance

- Cold lint: the walk is pruned to layer directories, files are read, parsed and resolved in parallel with a reused parse arena, and rule passes are linear (private-project warm run 1.8 s → 0.31 s).

### Removed

- `stratum-core`'s unused Salsa placeholder database and `stratum-plugins-rhai`'s unused `watch_plugins`.

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
