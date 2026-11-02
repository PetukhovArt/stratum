# Configuration

`stratum.config.jsonc` at the project root. Without it (zero-config), every `src/*` folder becomes a layer and the rules that need declared structure (`public-api`, `sibling-isolation`, `container-breadth`, `zone-breadth`, `missing-entry`) stay off; `stratum-lint init` writes the inferred config.

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
- `structure`: `entries` (default `index.ts`, `index.tsx`, `index.js`, `index.jsx`, `index.vue`) make a folder a container with one public entry; `segments` group a container's files by purpose; `private` folders (`stratum:recommended` sets `_shared`) are visible only under their parent.
- `overrides` change severity only. Unknown keys, slugs or options fail the run with exit 2.
- `.rhai` scripts add project-specific rules without rebuilding Stratum.
- `messages`: the project's own texts for `lint` output (below).

## Messages

Stratum prints English. A project that wants its output in another language, or in its own words, puts the texts in `messages`:

```jsonc
"messages": {
  "titles": { "stratum/no-circular-deps": "Цикл импортов" },
  "texts": {
    "Cycle": "Файлы через {name} импортируют друг друга (файлов в цикле: {count}). Чтобы разорвать цикл, достаточно убрать импортов: ~{closing}",
    "ListAnd": " и "
  }
}
```

- `texts` is keyed by catalog key. A key that is present replaces that English template, for messages, fixes and the pieces they are built from (list separators, parts of a sentence). A key that is absent stays English. A text must use the same `{placeholders}` as its English template.
- `titles` is keyed by rule slug, plugin rules included. A rule with a title prints `<title>. <message>`; a rule without one prints the message alone.
- `stratum-lint messages` lists every key with its English template and placeholders, and every built-in rule slug; `--format json` gives the same as JSON, a starting point for a translation.
- An unknown key, an unknown rule slug in `titles`, or a text whose placeholders differ from its English template's fails the run with exit 2 and names the key.
- `extends` merges `titles` and `texts` key by key, so a shared preset can carry a translation.
- Display only: every reporter (`terminal`, `agent`, `json`, `jsonl`, `sarif`, `gitlab`, `github`) prints the texts, but baseline entries and Code Quality fingerprints stay keyed on the English message, so adding or changing `messages` keeps every baseline entry and fingerprint. Text written in the config (`message`, `fix` of a rule option) and by `.rhai` rules is printed as written; rule docs (`explain`, `rules`) stay English.

Every rule, its options and its severity per preset: [custom-rules.md](custom-rules.md). It also shows a config translated from eslint-plugin-boundaries.
