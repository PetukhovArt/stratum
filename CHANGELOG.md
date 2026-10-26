# Changelog

## Unreleased

## 0.3.0 (2026-10-26)

- Added `ignore_type_only` and `ignore_dynamic` options to `stratum/no-circular-deps`: a cycle closed only by `import type` or by dynamic `import()` is no longer reported

## 0.2.0 (2026-10-18)

- Breaking: `--format json` is version 2: rules are named by slug (`stratum/no-circular-deps`, `custom/<name>`) and paths are relative to ROOT
- Breaking: config validation is strict: an unknown key, rule slug or option, a wrongly typed option, a `depends_on` naming an undeclared layer or a bad glob fails with exit 2 and names the key
- Breaking: `structure.private` defaults to `[]`; `_shared` comes from the `stratum:recommended` preset, so a config without `extends` that relied on it must set `"structure": { "private": ["_shared"] }`
- Breaking: stage is renamed to purity for the 1–4 rank: `// @stratum-purity N`, rule `stratum/purity`, Rhai `m.purity`; baseline entries keyed by the old slug go stale
- Breaking: `stratum/cross-entity-pattern` is replaced by `stratum/slice-fan-in` (option `max_fan_in`, off by default), and no folder name is hard-coded any more
- Breaking: calibrated defaults: `miller-limit` `max_children` 7 → 20, `deep-module` `max_exports` 12 → 20, `depth-ratio` `min_ratio` 3.0 → 1.0
- Breaking: Graph Snapshot v3 (ROOT-relative paths, private zones, territories); the Visualizer accepts only v3
- Breaking: `project.root` is removed, `overrides` set `severity` only, and the editor extensions for VS Code, Zed and WebStorm are removed (`stratum-lsp` stays)
- Breaking: "suggestion" is renamed to "fix" in Rhai (`fix`; `suggestion` still works with a deprecation warning), `/api/violations` and the terminal output; `--format json` v2 keeps the field name `suggestion`
- Added `--format jsonl`, the finding contract v1 for agents and harnesses: one object per finding, and every finding carries a non-empty `fix` (see `docs/harness.md`)
- Added `--format agent`, `--files`, `--changed[=REF]`, `--max` and `--baseline` / `--no-baseline` for end-of-turn gates
- Added `stratum-lint baseline [--prune]` and `baseline --check[=REF]`, so a legacy codebase reports only new violations
- Added exit code 3: no error in scope, but the baseline holds an entry fixed in scope; both CI templates treat it as a pass
- Added inline suppression `// stratum-ignore <slug> -- <reason>`, checked by the new `stratum/suppression` rule
- Added `stratum-lint diff --base REF`: the architecture drift of a merge request as markdown, with the imports that break a rule and new cycles
- Added `--format gitlab` (Code Quality) and `--format github` (annotations), and CI templates for GitLab and GitHub that run with no outbound internet (see `docs/ci.md`)
- Added `stratum-lint context <file>`: where a file, existing or planned, sits and what it may and may not import
- Added `stratum-lint explain <slug>` and `stratum-lint rules`, with built-in docs for every rule
- Added configurable structure: `layers[].slices` and `structure` (`entries`, `segments`, `private`); containers, their evolution stage and private zones come from the folder tree
- Added config `extends` with the presets `stratum:recommended` and `stratum:fsd`, `project.ignore`, `project.entry_points` and `stratum.schema.json`
- Added rules `public-api`, `sibling-isolation`, `import-policy`, `restricted-calls`, `stage-mixing`, `container-breadth`, `zone-breadth`, `missing-entry`, `no-barrel`, `composer-scope`, `naming` and `module-docs`
- Added rules `unused-exports` and `unreachable-modules`, which follow imports from `project.entry_points`
- Added an optional `fix` text to every `restricted-calls` entry and `import-policy` policy, shown as the finding's fix
- Added `restricted-calls` matching of member calls: `obj.name`, `*.name` and `ns.name()` on a namespace import
- Added `import.meta.glob('<pattern>')` edges, `.mts` / `.cts` sources and export counting, Vue `<script setup>` macros included
- Added Rhai `m.imports` (with `specifier`, `line`, `type_only`), `fn check(m, options)` and an optional `fn doc()`
- Added a graph cache (`--cache-dir`, `STRATUM_CACHE_DIR`): warm runs reparse only changed files, and findings never depend on the cache
- Added Claude Code plugins `stratum` (skill) and `stratum-lsp`, and `docs/agents-snippet.md` for Codex, Cursor and other agents; the npm package ships `stratum-lsp` too
- Added the rewritten Visualizer (SolidJS + SVG): Module focus, Matrix, Merge request, Container map, Module set and Bench views, EN / RU, fonts bundled for offline use
- Added `visualize --base REF`, `--config` and `--no-open`, and the `/api/context` and `/api/drift` endpoints
- Changed `stratum/no-circular-deps` to one finding per cycle with up to three imports to break, each with how many modules it frees
- Changed `stratum-lsp` to answer `initialize` at once, re-parse unsaved buffers in memory, honour the config and baseline, and rebuild when either changes
- Changed `deep-module`, `depth-ratio`, `miller-limit` and `promotion-pressure` to count real exports and importers, each importer once
- Changed `visibility-scope` to report imports into private zones; before, every module was public
- Changed zero-config to apply the `stratum:recommended` severities, with the rules that need declared structure off
- Changed the configured severity to cap a rule's findings: a rule may report lower, never higher
- Changed `miller-limit` to report at `info` by default
- Changed cold lint to run about 6× faster on a 4,000-module project (parallel walk and parse, linear rule passes)
- Fixed import findings pointing at line 1 instead of the import statement
- Fixed Vue `<script>` blocks with quoted attributes such as `generic="T extends X<Y>"` losing their body
- Fixed `diff --base` failing when an entry point does not exist at the base
- Removed the Visualizer's single-graph overview, replaced by the Container map

## 0.1.0 (2026-05-18)

- Added `stratum-lint` with `lint`, `init`, `snapshot`, `visualize` and `diff`
- Added 9 rules: `no-cross-layer-import`, `no-circular-deps`, `stage-purity`, `miller-limit`, `depth-ratio`, `visibility-scope`, `cross-entity-pattern`, `deep-module` and `promotion-pressure`
- Added TS / JS / JSX / TSX parsing via OXC, and Vue SFC `<script>` blocks
- Added `stratum-lsp` with hover, document links and diagnostics
- Added Rhai custom rules, terminal / JSON / SARIF output, `stratum.config.jsonc` with overrides and a JSON schema, and `--watch`
