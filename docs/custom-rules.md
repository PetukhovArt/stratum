# Custom rules

Three levels, from cheapest to most flexible: a preset via `extends`, the declarative `stratum/import-policy` rule, and Rhai scripts.
Editors and agents validate `stratum.config.jsonc` against [`stratum.schema.json`](../stratum.schema.json): add `"$schema": "<path or URL>/stratum.schema.json"`.

## `extends`: presets and shared configs

```jsonc
{ "version": 1, "project": { "extends": "stratum:fsd" } }
```

`extends` takes a file path relative to the extending config, or a built-in preset:

- `stratum:recommended`: the rules zero-config mode turns on.
- `stratum:fsd`: extends `stratum:recommended` and adds the Feature-Sliced Design layers `app → pages → widgets → features → entities → shared` under `src/<layer>`; each layer may import every layer below it.

A parent may extend further; a cycle is an error. Merge, child over parent:

| Key | Merge |
|---|---|
| `layers` | by `id`; a child entry overrides only the fields it sets, new ids are appended |
| `rules` | by slug; a child entry overrides only the fields it sets (`{ "severity": "off" }` keeps the parent's `options`) |
| `overrides` | parent blocks first, then the child's |
| `project.ignore` | parent globs first, then the child's |
| anything else | the child's value |

Errors fail the run with exit code 2: an unknown key, an unknown rule slug, an option the rule does not take or a wrongly typed one (the message names the slug and the key), a `depends_on` naming an undeclared layer, an invalid `project.ignore` glob.

## `project.ignore`: files outside the graph

```jsonc
"project": { "ignore": ["src/public/**", "**/__tests__/**", "**/*.test.ts", "!**/build"] }
```

Globs relative to the project root. An ignored file is not walked, parsed or put in the graph, and an import of it makes no edge, like an import of a package. A matching directory is skipped whole, so nothing under it can be un-ignored.
Built-in defaults come first in every config, zero-config included: `**/node_modules`, `**/dist`, `**/out`, `**/build`, `**/coverage`, `**/.git`, `**/*.d.ts`. The last matching glob wins; `!glob` un-ignores.
`overrides` only change severity: the files stay in the graph and still make cycles and importers. Use `ignore` for code that is not part of the architecture (vendored bundles, tests).

## `stratum/import-policy`: import boundaries without scripting

`options` is a list of policies. An import is reported once, with the message of the first policy that denies it.
Globs are matched against paths relative to the project root; `*` stays within one path segment, `**` crosses segments.

| Key | Meaning |
|---|---|
| `from` | importing files (required) |
| `except_from` | importing files the policy skips |
| `deny` | imported files (required) |
| `allow` | imported files exempt from `deny`, e.g. a slice's `index.ts` |
| `slice` | glob of slice directories; imports inside one slice are allowed |
| `message` | violation text; the imported path is appended |

```jsonc
"stratum/import-policy": {
  "severity": "error",
  "options": [
    {
      "from": ["src/features/**"],
      "deny": ["src/features/**"],
      "allow": ["src/features/*/index.ts"],
      "slice": "src/features/*",
      "message": "A feature imports another feature only through its index"
    }
  ]
}
```

Only imports that resolve to files inside a layer are in the graph: bare package imports (`vue`) cannot be denied, and type-only imports count as imports.
A full translation of an eslint-plugin-boundaries setup: [`examples/private-project.stratum.config.jsonc`](examples/private-project.stratum.config.jsonc).

## Rhai rules

Stratum lets you write project-specific rules in [Rhai](https://rhai.rs/) — a small embedded scripting DSL — without touching Rust or rebuilding the linter. A Rhai rule is a `.rhai` script that runs once per module and returns a list of violations.

## Quickstart (< 30 min)

1. Create `<your-rule>.rhai` somewhere inside your project, next to `stratum.config.jsonc` is conventional:

   ```rhai
   // no_legacy_imports.rhai
   fn check(m) {
       if m.path.contains("/legacy/") {
           return [
               #{
                   message: "Module is under src/legacy/ — phase out per migration plan.",
                   line: 1,
                   suggestion: "Move to the corresponding stable layer.",
               }
           ];
       }
       return [];
   }
   ```

2. Register the rule in `stratum.config.jsonc` under the conventional `custom/<slug>` namespace:

   ```jsonc
   {
     "version": 1,
     "layers": [ /* ... */ ],
     "rules": {
       "custom/no-legacy-imports": {
         "severity": "error",
         "script": "./no_legacy_imports.rhai"
       }
     }
   }
   ```

3. Run `stratum-lint .` — the violation will appear alongside built-in rule output.

## The `ModuleView` shape

The script defines `fn check(m)` or `fn check(m, options)`; a script with neither fails to load.
`m` is a `ModuleView` proxy with these fields.

| Field | Type | Description |
|-------|------|-------------|
| `m.id` | `int` | Stable numeric module id. |
| `m.path` | `string` | Absolute module path with forward slashes. |
| `m.layer` | `string` | The layer name from `stratum.config.jsonc`. |
| `m.container` | `int` | Stable numeric container id. |
| `m.purity` | `int` | Purity rank (1–4). |
| `m.dependents` | `array` of `int` | Module ids that depend on this module. |
| `m.imports` | `array` of `map` | Imports that resolve to a module in a layer: `#{ path, layer, kind }`, `kind` is `"static"`, `"di"` or `"runtime"`. Package imports are absent. |
| `m.metadata` | `map` | Reserved for future `@stratum-*` annotations; currently empty. |

`options` is the rule's `options` value from the config as a Rhai value (`()` when unset). Stratum does not validate it: check the keys you read.

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

## Documenting the rule

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

## The return shape

Return an `Array` of `Map` entries. Each map represents one violation:

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `message` | `string` | yes | The human-readable diagnostic message. |
| `line` | `int` | no (defaults to 1) | 1-based line number for the source location. |
| `suggestion` | `string` | no | Advice text — never an auto-applied fix (PRD decision #10). |

Return `[]` to mean "no violation". The `severity` is taken from `stratum.config.jsonc`; the script never sets it.

## Sandboxing

Stratum's Rhai engine is constructed with the Rhai standard package plus these explicit limits:

- `eval` is disabled.
- `max_expr_depths(64, 32)`, `max_call_levels(32)`.
- `max_operations(100_000)` — caps total instruction count per `check`.
- `max_string_size(8_192)`, `max_array_size(4_096)`.

Filesystem, network, and process access are **not** available — the standard Rhai package omits them by construction. If your rule needs project-wide data it does not see in `ModuleView`, file a feature request; we will not add `read_file` / `system` / `http` to the sandbox.

## Performance

Rhai scripts run once per `ModuleId`. As of Phase 9, no precompilation per-module is performed — each call re-pushes the `ModuleView` into a fresh scope. The PRD target of "Rhai overhead < 30% vs equivalent built-in" is not yet empirically gated; that benchmark is filed as a Phase 10 follow-up.

## Reloading

Today, the runtime loads each script once at `stratum-lint` startup. The Phase 4 `--watch` mode rebuilds the pipeline on file changes; a `.rhai` script change will be picked up on the next file event. Dedicated hot-reload (a `.rhai` save invalidating only the affected rule without rebuilding the graph) is a Phase 10 follow-up.

## Multiple custom rules

Each registered `custom/...` slug consumes one `RuleId` starting from `1000` (see `stratum_plugins_rhai::PLUGIN_ID_FLOOR`). The order in `stratum.config.jsonc` determines the id assignment. Built-in rule ids stay below `1000`; the gap reserved for built-ins prevents collisions.

A `script` path is relative to the project root, also when it comes from an extended config.
