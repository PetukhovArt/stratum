# Stratum

An architecture linter for TypeScript, JavaScript and Vue, built as the architecture check of an AI coding harness. It builds the project's dependency graph and checks it against units you configure: layers, slices inside layers, and the folder names that mark entries, segments and private zones. No folder name is hard-coded; FSD is one preset among others.

Source files: `.ts .tsx .mts .cts .js .jsx .mjs .cjs .vue` (Vue: `<script>` and `<script setup>` blocks).

Binaries: `stratum-lint` (CLI, visualizer server) and `stratum-lsp` (stdio language server).

## Install

- npm (all platforms): `npm install -g @petukhovart/stratum-lint` (ships both binaries)
- Windows: `scoop bucket add stratum https://github.com/PetukhovArt/scoop-stratum-bucket`, then `scoop install stratum-lint`
- macOS / Linux: `brew tap petukhovart/stratum`, then `brew install stratum-lint`
- From source: `cargo install --path crates/stratum-lint` and `cargo install --path crates/stratum-lsp`

## AI harness integration

- **Claude Code**: the `stratum` plugin is a skill that tells the agent when to run the CLI: `stratum-lint context` before placing a file, `stratum-lint --changed --format agent` before finishing. Stratum ships no hooks: wire the gate into your harness, pre-commit or CI. The optional `stratum-lsp` plugin adds warm LSP diagnostics, for projects without `typescript-lsp` only. See [claude-plugin](claude-plugin/README.md) and [claude-plugin-lsp](claude-plugin-lsp/README.md).
- **Codex, Cursor, other harnesses**: paste [docs/agents-snippet.md](docs/agents-snippet.md) into `AGENTS.md`, and launch `stratum-lsp` over stdio if the harness takes an LSP server.
- The CLI and LSP contract, exit codes and output shapes: [docs/harness.md](docs/harness.md).
- **CI** (GitLab, GitHub, offline): [docs/ci.md](docs/ci.md).

```
stratum-lint context src/features/cart/model/store.ts   # where a file sits, what it may import
stratum-lint --changed --format agent                    # end-of-turn gate: exit 1 on new errors
stratum-lint baseline                                    # accept current violations; later runs report only new ones
stratum-lint visualize                                   # architecture views in the browser
```

## Configuration

`stratum.config.jsonc` at the project root. Without it (zero-config), every `src/*` folder becomes a layer and the rules that need declared structure (`public-api`, `sibling-isolation`, `container-breadth`) stay off; `stratum-lint init` writes the inferred config.

```jsonc
{
  "$schema": "./stratum.schema.json",
  "version": 1,
  "project": {
    "extends": "stratum:recommended",               // or "stratum:fsd", or a relative file
    "ignore": ["**/__tests__/**", "**/*.test.ts"]   // out of the graph, on top of node_modules, dist, ...
  },
  "structure": { "segments": ["model", "ui", "api", "lib"], "private": ["_local"] },
  "layers": [
    { "id": "app",      "path": "src/app",      "depends_on": ["features", "shared"] },
    { "id": "features", "path": "src/features", "depends_on": ["shared"], "slices": true },
    { "id": "shared",   "path": "src/shared" }
  ],
  "rules": {
    "stratum/public-api": { "severity": "error" },
    "stratum/import-policy": { "severity": "error", "options": [
      { "from": ["src/shared/**"], "deny": ["src/shared/legacy/**"], "message": "legacy is frozen" }
    ] },
    "stratum/restricted-calls": { "severity": "error", "options": [
      { "calls": ["inject", "provide"], "allow": ["**/_local/*-scope.ts"], "message": "Use useXxxScope()" }
    ] },
    "custom/no-legacy-imports": { "severity": "warning", "script": "./rules/no_legacy_imports.rhai" }
  },
  "overrides": [
    { "files": ["src/app/**"], "rules": { "stratum/purity": { "severity": "off" } } }
  ]
}
```

- `layers`: a module belongs to the layer with the longest matching `path`; `depends_on` lists the layers it may import. With `slices: true`, each direct child of the layer is a slice.
- `structure`: `entries` (default `index.ts`, `index.tsx`, `index.js`, `index.jsx`, `index.vue`) make a folder a container with one public entry; `segments` group a container's files by purpose; `private` folders (default `_shared`) are visible only under their parent.
- `overrides` change severity only. Unknown keys, slugs or options fail the run with exit 2.
- `.rhai` scripts add project-specific rules without rebuilding Stratum.

Full reference: [docs/custom-rules.md](docs/custom-rules.md). A real-world config translated from eslint-plugin-boundaries: [docs/examples/private-project.stratum.config.jsonc](docs/examples/private-project.stratum.config.jsonc).

## Rules

`stratum-lint rules` lists the active ones with their severity; `stratum-lint explain <slug>` prints what, why, fix and options.

| Rule | Reports |
|---|---|
| `stratum/no-cross-layer-import` | An import from a layer outside the importer's own layer and its `depends_on`. |
| `stratum/no-circular-deps` | Every import cycle, one violation per cycle. |
| `stratum/purity` | A module importing a less pure one (`// @stratum-purity N`, 1 pure to 4 side effects, default 2). |
| `stratum/visibility-scope` | An import into a private-zone folder from outside that folder's parent. |
| `stratum/public-api` | An import from outside a slice, sub-unit or nested container that bypasses its entry file. |
| `stratum/sibling-isolation` | An import between two slices or sub-units that share a parent container. |
| `stratum/import-policy` | Imports denied by `from` / `deny` / `allow` / `slice` globs declared in config. |
| `stratum/restricted-calls` | A call to a listed function outside the files its policy allows. |
| `stratum/stage-mixing` | A container holding both segment folders and sub-unit folders. |
| `stratum/container-breadth` | A container or private zone with more than `max_kinds` kinds of children. |
| `stratum/slice-fan-in` | A slice imported by more than `max_fan_in` other slices. |
| `stratum/miller-limit` | A module with more than `max_children` importers from outside its slice (else its layer) whose layers may import it. |
| `stratum/promotion-pressure` | A module with at least `threshold` importers from outside its slice (else its layer) whose layers do not declare its layer: a candidate for a lower layer. |
| `stratum/deep-module` | A module exporting more than `max_exports` names. |
| `stratum/depth-ratio` | A module whose ratio of modules in its slice (else its layer) to its exports is below `min_ratio`. |

## More

- [docs/visualizer.md](docs/visualizer.md): the Visualizer, requirements and wishlist.
- [UBIQUITOUS_LANGUAGE.md](UBIQUITOUS_LANGUAGE.md): the glossary.
- [CHANGELOG.md](CHANGELOG.md), [RELEASE.md](RELEASE.md).
