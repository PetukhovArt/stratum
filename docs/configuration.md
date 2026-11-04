# Configuration

`stratum.config.jsonc` at the project root. Without it (zero-config), every `src/*` folder becomes a layer, with `slices: true` when at least half of its folders, and at least 2, hold an entry file; a folder name under at least 3 units of one layer and at least 30% of them becomes a segment, unless it is a private zone or a unit of that layer. The rules that need declared structure (`public-api`, `sibling-isolation`, `container-breadth`, `zone-breadth`, `missing-entry`) stay off, except that `public-api` and `sibling-isolation` keep their `stratum:recommended` severity once some layer is inferred sliced. `stratum-lint init` writes the inferred config: the full `structure`, the layers and every rule with its options.

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
  ],
  "features": {
    "checkout": { "name": "Checkout", "entry": ["src/pages/checkout/**"] },
    "search":   { "entry": ["src/features/search/index.ts"] }
  },
  "systems": [
    { "id": "ui", "name": "Web UI", "self": true },
    { "id": "host", "name": "Desktop host", "root": "../desktop/src",
      "contracts": { "kind": "rust-attr", "attr": "tauri::command" },
      "calls": [{ "callee": "invoke", "name_arg": 0 }] },
    { "id": "api", "name": "Backend", "root": "../backend/proto",
      "contracts": { "kind": "proto" },
      "calls": [{ "factory": "createClient", "service_arg": 0 }] }
  ]
}
```

- `layers`: a module belongs to the layer with the longest matching `path`; `depends_on` lists the layers it may import. With `slices: true`, each direct child of the layer is a slice.
- `structure`: `entries` (default `index.ts`, `index.tsx`, `index.js`, `index.jsx`, `index.vue`) make a folder a container with one public entry; `segments` group a container's files by purpose; `private` folders (`stratum:recommended` sets `_shared`) are visible only under their parent.
- `features`: a feature is its `entry` modules plus every module they reach by `import`, `export … from` and `require("…")`, type-only included; `import()` and `import.meta.glob()` are not followed. The id matches `^[\w.-]+$`, `name` defaults to the id, and each `entry` glob must match a module, else the run fails with exit 2. Without `features`, every folder or file directly in the first layer with `slices: true` is a feature named after it, a folder that another layer claims (a nested layer) included. A module of two or more features is shared. `stratum-lint features` prints them; `stratum-lint visualize --feature ID` opens the Visualizer filtered to them. A child config's `features` replaces the `extends` parent's.
- `systems`: the project is the one system with `"self": true`; every other system is something it calls. The id matches `^[\w.-]+$` and `name` defaults to the id. `root` is a directory relative to the project root, usually outside it, read for `contracts`:
  - `proto`: each `rpc M(Req) returns (Res)` of a `service S` in a `*.proto` file is the contract `S.M`; `stream`, option blocks, comments and an rpc split over lines are handled.
  - `rust-attr`: each `fn` in a `*.rs` file under `#[attr]` or `#[attr(…)]` is a contract named after the function; `#[command]` matches `attr: "tauri::command"` too, because the attribute is often imported by its last segment.
  - `calls` say how a module of the project names a contract. `{ "callee": "invoke", "name_arg": 0 }`: a call `invoke("name", …)` with a string literal at argument 0 is the contract `name`; `callee` takes `name`, `obj.name` and `*.name`, as in `stratum/restricted-calls`. `{ "factory": "createClient", "service_arg": 0 }`: `const c = createClient(UserService, …)` and then `c.getUser(…)` in the same module is the contract `UserService.GetUser`; the first letter of the method matches in either case. A client passed to another module, a factory call kept in no variable, or a name that is not a string literal is counted as unattributed.
  - A `root` that does not exist leaves the system without contracts, marked unavailable, and the run goes on. A call that matches no declared contract is listed without a declaration. `stratum-lint visualize` serves the result at `/api/systems`, and files under a `root` at `/api/source?path=id:rel/path`. A child config's `systems` replaces the `extends` parent's.
- `overrides` change severity only. Unknown keys, slugs or options fail the run with exit 2.
- `.rhai` scripts add project-specific rules without rebuilding Stratum.
- `messages`: the project's own texts for `lint` output (below).

## Messages

Stratum prints its built-in texts. A project that wants its own wording for messages and rule titles puts the texts in `messages`; they override the built-in text:

```jsonc
"messages": {
  "titles": { "stratum/no-circular-deps": "Import loop" },
  "texts": {
    "Cycle": "Files through {name} import each other ({count} files in the loop); removing ~{closing} imports breaks it",
    "ListAnd": " plus "
  }
}
```

- `texts` is keyed by catalog key. A key that is present overrides that built-in template, for messages, fixes and the pieces they are built from (list separators, parts of a sentence). A key that is absent keeps the built-in text. A text must use the same `{placeholders}` as its built-in template.
- `titles` is keyed by rule slug, plugin rules included. A rule with a title prints `<title>. <message>`; a rule without one prints the message alone.
- `stratum-lint messages` lists every key with its built-in template and placeholders, and every built-in rule slug; `--format json` gives the same as JSON, a starting point for a project's own wording.
- An unknown key, an unknown rule slug in `titles`, or a text whose placeholders differ from its built-in template's fails the run with exit 2 and names the key.
- `extends` merges `titles` and `texts` key by key, so a shared preset can carry a set of texts.
- Display only: every reporter (`terminal`, `agent`, `json`, `jsonl`, `sarif`, `gitlab`, `github`) prints the texts, but baseline entries and Code Quality fingerprints stay keyed on the built-in message, so adding or changing `messages` keeps every baseline entry and fingerprint. Text written in the config (`message`, `fix` of a rule option) and by `.rhai` rules is printed as written; rule docs (`explain`, `rules`) keep their built-in text.

Every rule, its options and its severity per preset: [custom-rules.md](custom-rules.md). It also shows a config translated from eslint-plugin-boundaries.
