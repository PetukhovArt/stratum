# Configuration and custom rules

`stratum.config.jsonc` at the project root (ROOT) configures everything; `--config PATH` points the CLI elsewhere, while `stratum-lsp` always reads `ROOT/stratum.config.jsonc`. Without a config (zero-config), every folder under `src/` becomes a layer, with no `depends_on` and no slices, and the `stratum:recommended` severities apply, except that `stratum/public-api`, `stratum/sibling-isolation` and `stratum/container-breadth` are off: without declared layers and structure they flood the first run. `stratum/visibility-scope` stays on, since private zones are found by folder name. `stratum-lint init` writes the inferred config.
Editors and agents validate the file against [`stratum.schema.json`](../stratum.schema.json): add `"$schema": "<path or URL>/stratum.schema.json"`.

Customization goes from cheapest to most flexible: a preset via `extends`, `layers` and `structure`, rule `severity` / `options` and `overrides`, the declarative `stratum/import-policy` and `stratum/restricted-calls` rules, and Rhai scripts.

## Top-level keys

| Key | Meaning |
|---|---|
| `version` | required, `1` |
| `project.extends` | a config file relative to this one, or a preset: `stratum:recommended`, `stratum:fsd` |
| `project.ignore` | globs of files kept out of the graph |
| `project.entry_points` | globs of the files the program starts from; required by `stratum/unused-exports` and `stratum/unreachable-modules` |
| `structure` | folder and file names that shape containers |
| `layers` | `[{ id, path, depends_on?, slices? }]` |
| `rules` | by slug: `{ severity, options?, script? }` |
| `overrides` | `[{ files, rules: { <slug>: { severity } } }]` |

Errors fail every command that loads the config with exit code 2 and are reported by `stratum-lsp` via `window/showMessage`: an unknown key at any level (`project.root` included), an unknown rule slug in `rules` or `overrides`, an option the rule does not take or a wrongly typed one (the message names the slug and the key), `options` or `script` in an override, a `depends_on` naming an undeclared layer, an invalid `project.ignore` or `overrides[].files` glob, an unknown preset, an `extends` cycle.

## Layers and slices

```jsonc
"layers": [
  { "id": "app",      "path": "src/app",      "depends_on": ["features", "shared"] },
  { "id": "features", "path": "src/features", "depends_on": ["shared"], "slices": true },
  { "id": "shared",   "path": "src/shared" }
]
```

- `path` is relative to ROOT. A file belongs to the layer with the longest matching path; a file outside every layer is not in the graph and nothing reports it.
- `depends_on` lists the layers this layer may import (`stratum/no-cross-layer-import`). A layer may always import itself.
- `slices: true` makes each direct child of the layer folder a slice, folder or source file. Slices of one layer may not import each other (`stratum/sibling-isolation`) and are reached from outside only through their entry (`stratum/public-api`).

## `structure`: containers from the folder tree

```jsonc
"structure": {
  "entries":  ["index.ts", "index.tsx", "index.js", "index.jsx", "index.vue"],  // default
  "segments": [],                                                              // default; stratum:fsd sets model, ui, api, lib, config
  "private":  []                                                               // default; stratum:recommended sets _shared
}
```

Names match whole path components, never substrings. Containers are derived from these names and the file set, never declared one by one:

- Each layer is a container. An entry file directly in the layer folder is the layer's entry, never a slice.
- In a `slices: true` layer, each direct child is a slice. A file slice is its own entry.
- Below a slice, sub-unit or nested container, a direct child folder that holds an entry file is a **sub-unit**: isolated from its sibling sub-units, reached from outside only through its entry.
- An entry-bearing folder deeper down (under a segment or collection folder, or directly under a layer without `slices`) is a **nested** container: reached through its entry, not isolated from its siblings.
- A **segment** folder (`segments`) groups a container's files by purpose and is never a container. A folder without an entry file is a **collection** folder: its files belong to the enclosing container.
- A **private zone** (`private`) is never a container. A module under `D/<zone>/...` may be imported only from under `D` (`stratum/visibility-scope`); when zones nest, the innermost wins.
- Each container has a stage: 1 for a file slice, 4 when it has a sub-unit child, 3 when it has a segment folder child, 2 otherwise. Private zones count toward neither.

Placement depends only on the set of file paths: adding an `index.ts` turns a collection folder into a container. `stratum-lint context <file>` prints the container, stage and entry a file gets.

## `extends`: presets and shared configs

```jsonc
{ "version": 1, "project": { "extends": "stratum:fsd" } }
```

- `stratum:recommended`: sets `structure.private` to `_shared` and the severities in the table below. `deep-module`, `promotion-pressure` and `import-policy` are not listed and run at their default severity unless set to `"off"`.
- `stratum:fsd`: extends `stratum:recommended`; adds the Feature-Sliced Design layers `app → pages → widgets → features → entities → shared` under `src/<layer>`, each depending on every layer below it, with `slices: true` on pages, widgets, features and entities; sets `structure.segments` to `model, ui, api, lib, config`; tightens the severities in the fsd column.

A parent may extend further; a cycle is an error. Merge, child over parent:

| Key | Merge |
|---|---|
| `layers` | by `id`; a child entry overrides only the fields it sets (`path`, `depends_on`, `slices`), new ids are appended |
| `rules` | by slug; a child entry overrides only the fields it sets (`{ "severity": "off" }` keeps the parent's `options`) |
| `overrides` | parent blocks first, then the child's |
| `project` | field by field; `ignore` appends: parent globs first, then the child's |
| `structure` | field by field; a child list replaces the parent's list |
| anything else | the child's value |

## Rules, severities and options

`rules` is keyed by slug. `severity` is `error`, `warning`, `info` or `off`; a built-in absent from the merged config runs at its default severity. Only `error` makes `lint` exit 1.

| Slug | Default | recommended | fsd | Options (default) |
|---|---|---|---|---|
| `stratum/no-cross-layer-import` | error | warning | warning | none |
| `stratum/no-circular-deps` | error | error | error | none |
| `stratum/purity` | error | warning | warning | none |
| `stratum/visibility-scope` | error | error | error | none |
| `stratum/public-api` | warning | warning | error | none |
| `stratum/sibling-isolation` | warning | warning | error | none |
| `stratum/import-policy` | error | | | list of policies, below |
| `stratum/restricted-calls` | off | off | off | list of policies, below |
| `stratum/stage-mixing` | off | off | error | none |
| `stratum/container-breadth` | info | info | warning | `max_kinds` (6), `max_items` (20), `max_root_files` (4) |
| `stratum/zone-breadth` | warning | warning | error | `max_kinds` (6) |
| `stratum/missing-entry` | warning | warning | error | none |
| `stratum/no-barrel` | warning | warning | error | `layers` ([]) |
| `stratum/composer-scope` | warning | warning | warning | `scope_file` (unset) |
| `stratum/naming` | warning | off | warning | `folder_case` (unset), `reserved` ([]), `duplicate_slices` (false) |
| `stratum/module-docs` | warning | warning | warning | `nav_file` (unset), `requires_key` ("requires") |
| `stratum/unused-exports` | off | off | off | none |
| `stratum/unreachable-modules` | off | off | off | none |
| `stratum/suppression` | error | error | error | none |
| `stratum/slice-fan-in` | off | off | warning | `max_fan_in` (5) |
| `stratum/miller-limit` | warning | info | info | `max_children` (20) |
| `stratum/promotion-pressure` | info | | | `threshold` (5) |
| `stratum/deep-module` | info | | | `max_exports` (20) |
| `stratum/depth-ratio` | info | info | info | `min_ratio` (1.0) |

The metric rules' limits are calibrated on a real project, see [calibration.md](calibration.md); none of them is ever `error`. The fsd preset also sets `no-barrel` `layers: ["shared"]`, `composer-scope` `scope_file: "*-scope.*"`, and `naming` `folder_case: "kebab"`, `reserved: ["hooks", "helpers", "types", "utils", "components"]`, `duplicate_slices: true`. `unused-exports` and `unreachable-modules` are off in both presets; set `project.entry_points` and turn them on at `warning`. Turning either on without entry points is a config error.

`stratum-lint explain <slug>` prints what a rule checks, why, how to fix a hit, and its options; `stratum-lint rules` lists the effective severity of every rule.
`// @stratum-purity N` (1 pure to 4 side effects, default 2) in a file's leading comments sets its purity for `stratum/purity`.

The configured severity is a cap: a rule may report a finding at a lower severity, never a higher one. `container-breadth` reports private-zone folders at `info` at most.

## `overrides`: severity by glob

```jsonc
"overrides": [
  { "files": ["src/legacy/**", "**/*.stories.ts"], "rules": { "stratum/no-cross-layer-import": { "severity": "off" } } }
]
```

- `files` are globs matched against forward-slash paths relative to ROOT.
- An override sets `severity` only; `options` and `script` come from the `rules` entry. Every slug must exist.
- Blocks apply in order; for each rule the last matching block wins. The file is the violation's file; `stratum/no-circular-deps` reports whole cycles, so only its project-wide severity applies.
- Overridden files stay in the graph and still make cycles and importers. Use `project.ignore` for code that is not part of the architecture.

## `project.ignore`: files outside the graph

```jsonc
"project": { "ignore": ["src/public/**", "**/__tests__/**", "**/*.test.ts", "!**/build"] }
```

Globs relative to ROOT. An ignored file is not walked, parsed or put in the graph, and an import of it makes no edge, like an import of a package. A matching directory is skipped whole, so nothing under it can be un-ignored.
Built-in defaults come first in every config, zero-config included: `**/node_modules`, `**/dist`, `**/out`, `**/build`, `**/coverage`, `**/.git`, `**/*.d.ts`. The last matching glob wins; `!glob` un-ignores.

## `stratum/import-policy`: import boundaries without scripting

`options` is a list of policies. An import is reported once, with the message of the first policy that denies it.
Globs are matched against paths relative to ROOT; `*` stays within one path segment, `**` crosses segments.

| Key | Meaning |
|---|---|
| `from` | importing files (required) |
| `except_from` | importing files the policy skips |
| `deny` | imported files (required) |
| `allow` | imported files exempt from `deny`, e.g. a slice's `index.ts` |
| `slice` | glob of directories; an import between two files under the same matching directory is allowed |
| `message` | violation text; the imported path is appended |

```jsonc
"stratum/import-policy": {
  "severity": "error",
  "options": [
    { "from": ["src/shared/**"], "deny": ["src/shared/legacy/**"], "message": "legacy is frozen" }
  ]
}
```

Only imports that resolve to files inside a layer are in the graph: bare package imports (`vue`) cannot be denied, and type-only imports count as imports.
Slice isolation and entry-only access need no policy: declare `slices: true` and use `stratum/sibling-isolation` and `stratum/public-api`. A full translation of an eslint-plugin-boundaries setup: [`examples/private-project.stratum.config.jsonc`](examples/private-project.stratum.config.jsonc).

## `stratum/restricted-calls`: calls allowed only in chosen files

`options` is a list of policies. A call is reported, at its line, by the first policy that lists its name and does not allow the calling file.

| Key | Meaning |
|---|---|
| `calls` | function names (required) |
| `allow` | globs of files where these calls are allowed, relative to ROOT |
| `message` | violation text; the called name is appended |

```jsonc
"stratum/restricted-calls": {
  "severity": "error",
  "options": [
    { "calls": ["inject", "provide"], "allow": ["**/_shared/*-scope.ts"], "message": "Use useXxxScope()/provideXxxScope()" }
  ]
}
```

A plain identifier callee matches, and so does a local alias of an imported listed name (`import { inject as i }`, then `i()`). Member calls such as `x.inject()` do not. The parser collects calls only when the `rules` entry has options and the rule is on there or in some override.

## Rhai rules

A Rhai rule is a `.rhai` script, in the small embedded [Rhai](https://rhai.rs/) language, that runs once per module and returns a list of violations. No Rust and no rebuild of the linter.

1. Write the script, conventionally next to `stratum.config.jsonc`:

   ```rhai
   // no_legacy_imports.rhai
   fn check(m) {
       let out = [];
       for i in m.imports {
           if i.path.contains("/legacy/") {
               out.push(#{ message: "imports legacy module " + i.path, fix: "Use the replacement in shared/." });
           }
       }
       out
   }
   ```

2. Register it under a slug, conventionally `custom/<name>`:

   ```jsonc
   "rules": {
     "custom/no-legacy-imports": { "severity": "error", "script": "./no_legacy_imports.rhai" }
   }
   ```

   The `script` path is relative to ROOT, also when it comes from an extended config.

3. Run `stratum-lint`: the violation appears next to the built-in ones, named by the slug.

### `check` and the `ModuleView`

The script defines `fn check(m)` or `fn check(m, options)`; a script with neither fails to load (exit 2).
`options` is the rule's `options` value from the config as a Rhai value (`()` when unset). Stratum does not validate it: check the keys you read.

`m` is a read-only `ModuleView`:

| Field | Type | Description |
|-------|------|-------------|
| `m.id` | `int` | Module id. |
| `m.path` | `string` | Absolute module path with forward slashes. |
| `m.layer` | `string` | The layer `id` from the config. |
| `m.container` | `int` | Id of the innermost container. Deterministic for a given file set, not stable when files are added or removed. |
| `m.purity` | `int` | Purity rank, 1–4. |
| `m.dependents` | `array` of `int` | Ids of the modules that import this one. |
| `m.imports` | `array` of `map` | Imports that resolve to a module in a layer: `#{ path, layer, kind, specifier, line, type_only }`, with `path` absolute, `kind` one of `"static"`, `"di"`, `"runtime"`, `specifier` as written in source and `line` the 1-based line of the import statement. Package imports are absent. |
| `m.metadata` | `map` | Reserved; empty. |

```rhai
fn check(m, options) {
    let out = [];
    for i in m.imports {
        if options.banned_layers.contains(i.layer) {
            out.push(#{ message: "imports " + i.path + " from layer " + i.layer });
        }
    }
    out
}
```

### The return shape

Return an `Array` of `Map` entries, one per violation; `[]` means none. Entries that are not maps are skipped.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `message` | `string` | yes | The diagnostic message. |
| `line` | `int` | no (defaults to 1) | 1-based line of the source location. |
| `fix` | `string` | no | Advice text, never an auto-applied edit. The old key `suggestion` is still read, with a deprecation warning on stderr once per rule. |

The severity comes from the config; the script never sets it. A `check` call that fails at runtime logs `[rhai-rule <slug>] call_fn failed: ...` to stderr and reports nothing for that module.

### `doc()`

An optional `fn doc()` returning a map makes `stratum-lint rules` and `explain` show the rule like a built-in. It runs once at load; a wrong shape fails the load.

```rhai
fn doc() {
    #{
        summary: "No imports from banned layers",        // required
        why: "Those layers are being phased out.",
        fix: "Import the replacement from shared/.",
        options: ["banned_layers: layer names to deny ([])"],
    }
}
```

### Sandbox, loading, ids

- The engine is Rhai's standard package with `eval` disabled, `max_expr_depths(64, 32)`, `max_call_levels(32)`, `max_operations(100_000)` per call, `max_string_size(8_192)` and `max_array_size(4_096)`. Filesystem, network and process access are not available. `print` and `debug` go to stderr.
- The CLI loads each script at startup; `--watch` reloads it on the next file event. `stratum-lsp` watches every script referenced from the config and rebuilds when one changes.
- Scripts run once per module. No benchmark gates their overhead yet.
- Plugin rules get internal ids from `1000` up, in alphabetical slug order; outputs only ever show slugs.
