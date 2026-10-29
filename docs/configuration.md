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
- `features`: a feature is its `entry` modules plus every module they reach by `import`, `export … from` and `require("…")`, type-only included; `import()` and `import.meta.glob()` are not followed. The id matches `^[\w.-]+$`, `name` defaults to the id, and each `entry` glob must match a module, else the run fails with exit 2. Without `features`, every slice of the first layer with `slices: true` is a feature named after the slice. A module of two or more features is shared. `stratum-lint features` prints them; `stratum-lint visualize --feature ID` opens the Visualizer filtered to them. A child config's `features` replaces the `extends` parent's.
- `systems`: the project is the one system with `"self": true`; every other system is something it calls. The id matches `^[\w.-]+$` and `name` defaults to the id. `root` is a directory relative to the project root, usually outside it, read for `contracts`:
  - `proto`: each `rpc M(Req) returns (Res)` of a `service S` in a `*.proto` file is the contract `S.M`; `stream`, option blocks, comments and an rpc split over lines are handled.
  - `rust-attr`: each `fn` in a `*.rs` file under `#[attr]` or `#[attr(…)]` is a contract named after the function; `#[command]` matches `attr: "tauri::command"` too, because the attribute is often imported by its last segment.
  - `calls` say how a module of the project names a contract. `{ "callee": "invoke", "name_arg": 0 }`: a call `invoke("name", …)` with a string literal at argument 0 is the contract `name`; `callee` takes `name`, `obj.name` and `*.name`, as in `stratum/restricted-calls`. `{ "factory": "createClient", "service_arg": 0 }`: `const c = createClient(UserService, …)` and then `c.getUser(…)` in the same module is the contract `UserService.GetUser`; the first letter of the method matches in either case. A client passed to another module, a factory call kept in no variable, or a name that is not a string literal is counted as unattributed.
  - A `root` that does not exist leaves the system without contracts, marked unavailable, and the run goes on. A call that matches no declared contract is listed without a declaration. `stratum-lint visualize` serves the result at `/api/systems`, and files under a `root` at `/api/source?path=id:rel/path`. A child config's `systems` replaces the `extends` parent's.
- `overrides` change severity only. Unknown keys, slugs or options fail the run with exit 2.
- `.rhai` scripts add project-specific rules without rebuilding Stratum.

Every rule, its options and its severity per preset: [custom-rules.md](custom-rules.md). It also shows a config translated from eslint-plugin-boundaries.
