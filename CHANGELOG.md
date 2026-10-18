# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The **Graph Snapshot** version is the on-wire contract between `stratum-lint`
(`snapshot`, `visualize`) and the Visualizer. It is tracked separately from the
package version; a bump requires a coordinated frontend release.

## Unreleased

### Breaking

- Calibrated defaults: `stratum/miller-limit` `max_children` 7 to 20, `stratum/deep-module` `max_exports` 12 to 20, `stratum/depth-ratio` `min_ratio` 3.0 to 1.0.
- `structure.private` defaults to `[]`; `_shared` now comes from the `stratum:recommended` preset (so from `stratum:fsd`, zero-config and `init`). A config that has no `extends` and relied on the old default must set `"structure": { "private": ["_shared"] }`.
- `--format json` is version 2: `{ "version": 2, "summary": {...}, "violations": [...] }`. Rules are named by slug (`stratum/no-circular-deps`, `custom/<name>`), never by numeric id, and paths are relative to ROOT.
- Graph Snapshot v3. Over v2 (`module.stage` → `module.purity`; containers with `path`, `kind` (`layer`, `slice`, `sub_unit`, `nested`), `stage` (the evolution stage, 1–4) and `entry`) it adds `project` (the root folder name), `zones [{ id, path, root }]` (private zones), `territories` (what a module may import, each distinct one once) and per module `zone` (absent outside a zone) and `territory` (an index). Every path is ROOT-relative with `/`: no absolute machine path is left in the snapshot. The Visualizer accepts only v3.
- Stage is renamed to purity for the 1–4 rank: `// @stratum-stage N` is now `// @stratum-purity N` and the old annotation is no longer recognised; `stratum/stage-purity` is now `stratum/purity`; Rhai `m.stage` is `m.purity`; `context` prints `purity`. "Stage" now means only a container's evolution stage. Baseline entries keyed by the old slug go stale.
- `stratum/cross-entity-pattern` is replaced by `stratum/slice-fan-in`: distinct importing slices per slice, option `max_fan_in` (5, was `max_fanout`), off by default. No folder name (`entities`, `features`, `index.ts`, `_shared`) is hard-coded any more; slices come from `layers[].slices`.
- `project.root` is removed: nothing read it, the graph always walks ROOT. It is now an unknown key, and `project` is optional.
- Config validation is strict: an unknown key, an unknown rule slug, an option the rule does not take or a wrongly typed one, a `depends_on` naming an undeclared layer, a bad `project.ignore` or `overrides[].files` glob, an unknown preset or an `extends` cycle fails with exit 2, naming the slug and key. `lint`, `baseline`, `context`, `rules`, `explain`, `snapshot` and `visualize` exit 2 on it, as on any other tool failure.
- `overrides` set `severity` only; `options` or `script` in an override is a config error.
- Editor extensions are removed (`editor-extensions/` for VS Code, Zed, WebStorm). `stratum-lsp` stays and targets AI harnesses: see `docs/harness.md`.
- `stratum-lint diff` is rewritten: it was a stub that always failed, and now takes `--base REF` (required) and prints a drift report.
- Suggestion is renamed to fix: the Rhai return key `fix` (a script returning `suggestion` still works and gets a deprecation warning on stderr once per rule), `/api/violations` field `fix`, the terminal `fix:` line. `--format json` v2 keeps the field name `suggestion`.
- `stratum/restricted-calls`: a `calls` entry that is not `name`, `obj.name` or `*.name` is a config error.

### Added

- Rules `stratum/zone-breadth`, `stratum/missing-entry`, `stratum/no-barrel`, `stratum/composer-scope`, `stratum/naming`, `stratum/module-docs`, `stratum/unused-exports`, `stratum/unreachable-modules` and `stratum/suppression` are registered with their docs, options and preset severities; their checks arrive in later changes, so they report nothing yet. `stratum/container-breadth` declares `max_items` (20) and `max_root_files` (4). Config `project.entry_points` (globs); enabling `unused-exports` or `unreachable-modules` without it is a config error. `docs/calibration.md` records the sweeps behind the metric defaults.
- `stratum/unreachable-modules` reports each module no entry point reaches through any import. `stratum/unused-exports` reports each export of a reachable module that no reachable module imports, following names through `export { x } from` and `export * from`; entry points' exports and an imported `.vue` file's default export count as used. Both are off in the presets. A `project.entry_points` glob that matches no module is a tool failure (exit 2) naming the glob. Blind spots are in `docs/custom-rules.md`.
- `stratum/module-docs` checks navigation docs: with `nav_file` set, a layer root, container or private zone without it, one inside a segment or collection folder, and a `requires` entry in its frontmatter (`requires_key`) whose folder imports the requiring folder or is its sibling container. The engine reads the docs of the folders in `dir_roles` into `graph.nav_docs`; inert without `nav_file`.
- Every `stratum/restricted-calls` entry and every `stratum/import-policy` policy accepts an optional `fix` (what to use instead: module and export); it becomes the finding's `fix`, otherwise the rule's generic fix applies.
- Gate options: `--format agent` (compact text for a model's context), `--max`, `--files`, `--changed[=REF]`, `--baseline` / `--no-baseline`. `stratum-lint baseline [--prune]` writes `stratum.baseline.json`, fingerprinted by slug, file and message with numbers masked.
- CI integration, see `docs/ci.md`:
  - `--format gitlab` (Code Quality JSON with line-shift-stable fingerprints) and `--format github` (annotations capped at 10 per severity, then the agent text). Their paths are relative to the git work-tree root.
  - `stratum-lint baseline --check[=REF]`: exit 1 when a baseline entry count grew since REF; reads no config, builds no graph.
  - `stratum-lint diff --base REF`: architecture drift of the working tree vs REF as capped markdown, first line `<!-- stratum:drift -->`; exit 1 on drift, meant as advisory.
  - The drift report lists imports added since REF that break a built-in import rule (`Imports that break rules`, by slug and severity); a new cycle carries an ordered path through an added import.
  - CI Templates `ci/gitlab/templates/stratum.yml` (`include: project:`) and `ci/github` (composite action): gate, ratchet (skipped with the label `stratum:accept-debt`) and a drift note / comment, with no outbound internet.
- `stratum-lint context <file>`: where a file, existing or planned, sits and what it may import: layer, purity, container with kind, stage and entry, the enclosing slice, visible private zones, and a `may not import` line (sibling slices, sibling sub-units, files behind another container's entry).
- `stratum-lint explain <slug>` and `stratum-lint rules`, with built-in docs for every rule.
- Configurable structure: `layers[].slices` and a `structure` key (`entries`, `segments`, `private`). Containers (layer, slice, sub-unit, nested), their evolution stage and private zones are derived from the folder tree.
- Rules `stratum/public-api`, `stratum/sibling-isolation`, `stratum/restricted-calls` (with call tracking in the parser), `stratum/stage-mixing`, `stratum/container-breadth`, and the declarative `stratum/import-policy`.
- Config `extends`: a relative file or the presets `stratum:recommended` and `stratum:fsd`, merged by layer id and rule slug. `project.ignore` keeps files out of the graph, on top of built-in defaults. `stratum.schema.json`, referenced with `"$schema"`.
- Rhai plugins: `m.imports`, `fn check(m, options)`, an optional `fn doc()` that shows the rule in `rules` / `explain`; `print` and `debug` go to stderr.
- `.mts` / `.cts` sources. Export counting (`Exports`), including Vue `<script setup>` macros.
- Claude Code plugins `stratum` (skill) and `stratum-lsp` (LSP), with a marketplace; `docs/agents-snippet.md` for Codex, Cursor and others. The npm wrapper ships `stratum-lsp` too.
- `stratum-lint visualize --config PATH` and `--no-open` (serve without launching the browser). `stratum-lint snapshot --config PATH`.
- `stratum-lint visualize --base REF` serves the **Drift** vs REF at `/api/drift` (with the `diff` markdown); without `--base` it is 404 and the Merge request view is hidden.
- Visualizer endpoint `GET /api/context?path=REL`: the file's brief as `text/plain`, byte for byte what `stratum-lint context REL --format agent` prints.
- Visualizer: rewritten in SolidJS + SVG (PixiJS and dagre dropped) as views, each with its own outline and details panel, switched by tab, palette (Ctrl+K) or key; `v` cycles a view's variant, and navigation persists in `localStorage`. See `docs/visualizer.md`.
  - `1` Module focus: a module's importers and imports in layer columns (with a second hop) or bands, its **Territory** strip, its brief, SVG export, "Add to set".
  - `2` Matrix: a DSM of layers or units, partitioned into cycle blocks, cells against `depends_on`; "Show in map" opens the cell's pair in the Container map.
  - `3` Merge request (with `--base`): the changed region with added and removed imports and the files behind them, or a comment card ≤ 800×600 next to the `diff` markdown, exportable as SVG.
  - `4` Container map: Lanes + bundles (units per layer lane, bundled unit imports gated by rule breaks, ≥ 12 imports or all; double-click expands a unit into its modules) or a Layer graph of declared and undeclared layer imports.
  - `5` Module set: a set of modules, from "Add to set" or a module cycle, as a Ring, an Arc diagram or a Set matrix, with the modules on short paths outside the set and the cut that frees the most modules from the cycle (its cut score).
  - `9` Bench: canonical questions per view and render runs against gates (≤ 1,500 marks, pan/zoom p95 ≤ 16.7 ms, severity colour only on severity marks, edge rules matching lint, card ≤ 800×600); `BENCH_URL=<url> npx playwright test tests/e2e/bench.spec.ts` runs it on a live `visualize`.
  - UI language EN | RU; Inter Tight and JetBrains Mono (Latin and Cyrillic) bundled, so it works offline.
- Breadth checks are live. `stratum/container-breadth` also reports a segment, collection or sliced layer root with more than `max_items` items, a stage-4 container with more than `max_root_files` files at its root, and a single-file folder (at `info` at most). The private-zone check moved to `stratum/zone-breadth`, which now reports at the configured severity instead of `info`.
- Shape checks are live: `stratum/missing-entry`, `stratum/no-barrel`, `stratum/composer-scope` and `stratum/naming` report; `stratum/stage-mixing` also reports a plain domain folder next to segment folders (layers are exempt). All of them check every folder at any depth and give each finding a concrete fix. `DirRole::Segment` carries the entry file sitting in it.
- Rhai `m.imports` entries carry `specifier`, `line` and `type_only`.
- `stratum/restricted-calls` entries: `name` also matches `ns.name()` on a namespace import, `obj.name` matches a member call on `obj`, `*.name` any member call.
- `import.meta.glob('<pattern>')` with a relative or `/`-rooted literal pattern makes runtime edges to every matching module.
- `--format jsonl`, the finding contract v1: one `{"v":1,"tool":"stratum","rule","severity","file","line","message","fix"}` object per line, nothing else on stdout, a clean run prints nothing. `fix` is never empty: the violation's own text, else the rule doc's fix, else `Run "stratum-lint explain <rule>"`. See `docs/harness.md`.
- Exit code 3 of `lint`: no error in scope, but the baseline holds an entry fixed in scope (stale baseline); prune it with `baseline --prune` in a change of its own. Precedence 2, 1, 3, 0. `--no-baseline` never returns 3. Both CI Templates treat 3 as a pass.
- Inline suppression `// stratum-ignore <slug>[, <slug>] -- <reason>` (next line only), checked by `stratum/suppression`: a missing reason or an unknown slug is an error and suppresses nothing, a directive that hides nothing is a warning. Applied before the baseline. `--format json` summary gains `suppressed`, the agent summary `, N suppressed`; `jsonl`, `gitlab` and `github` omit suppressed findings, `sarif` keeps them with `suppressions`.
- `docs/harness.md` budgets table and `eval/` fixtures (`eval/README.md`) with the runner `crates/stratum-lint/tests/eval.rs`.
- Graph Cache: every graph built from disk (lint, baseline, context, snapshot, visualize, the LSP Server's full rebuilds) reuses the parsed facts and resolved imports of unchanged files from `DIR/stratum-<hash>.graph`. `--cache-dir DIR` on every command (env `STRATUM_CACHE_DIR`), default the OS user cache dir + `stratum`. Findings do not depend on what the cache holds. The `--format json` summary gains `cache: { state: "warm" | "cold" | "off", reparsed }`. See `docs/harness.md#graph-cache`.
- `stratum-budget` (internal, not published): `generate` writes a deterministic 4,000-module project, `check` measures `stratum-lint` p50/p95 and peak RSS in the four scenarios of the `docs/harness.md` budgets table and exits 1 over budget; run before each release. `--files` and warm-versus-cold equivalence tests in `crates/stratum-lint/tests/e2e_equivalence.rs`.

### Changed

- Every finding carries a non-empty fix, so `--format json` v2 `suggestion` is never `null` and the terminal and agent formats print a `fix:` line for every rule.

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
- `--format agent` collapses info findings into one advisory line.
- `/api/violations` carries rule slugs and ROOT-relative paths.
- `context` is more accurate, with the brief version unchanged: the sibling line appears only when an isolated sibling exists, and a rule turned off for the file by an override lifts its limit (cross-layer off: every layer; purity off: purity <= 4; visibility off: every private zone).
- `stratum/no-circular-deps` reports one finding per cycle at its anchor (the member with the smallest path) with a message of at most 240 characters: `Import cycle of N modules through <anchor>; about k imports close it`. It no longer lists every member; the fix names up to three imports to break, best first, each with how many modules it frees from the cycle. `--format agent` prints the fix under each example when the fixes in a group differ. Baseline entries for the old `Circular dependency: …` message go stale.

### Fixed

- Vue: a `<script>` tag end respects quoted attributes, so `generic="T extends X<Y>"` keeps the script body.
- Import findings (`no-cross-layer-import`, `public-api`, `sibling-isolation`, `visibility-scope`, `purity`, `import-policy`) point at the line of the import statement instead of line 1.
- `stratum-lint diff --base REF` no longer exits 2 when a `project.entry_points` glob matches nothing at the base: entry points are selected at the checked tree only, drift being advisory.

### Performance

- Cold lint: the walk is pruned to layer directories, files are read, parsed and resolved in parallel with a reused parse arena, and rule passes are linear (warm run 1.8 s → 0.31 s on a ~4K-module project).
- The directory walk runs in parallel and takes size and mtime from the directory entries; `import-policy` matches every module against all policies once per run instead of per import; `--changed` (REF = HEAD) runs one `git status` instead of `git diff` + `git ls-files`, and for other REFs runs the two in parallel.

### Removed

- `stratum-core`'s unused Salsa placeholder database and `stratum-plugins-rhai`'s unused `watch_plugins`.
- Visualizer: the single-graph overview of 0.2.0-rc.1 (fractal pack layout) is replaced by the Container map, and with it go the facet bar, the colour modes and stage / purity filters, the path glob filter, the violators-only filter, the minimap, the overview legend, hover tooltips with violation lists, the per-layer and per-container visibility toggles and cycle focus.

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
