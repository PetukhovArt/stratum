# Custom rules in Rhai

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
     "project": { "root": "./src" },
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

`check(m)` receives one argument: a `ModuleView` proxy with these fields.

| Field | Type | Description |
|-------|------|-------------|
| `m.id` | `int` | Stable numeric module id. |
| `m.path` | `string` | Forward-slash module path, relative to the project root when possible. |
| `m.layer` | `string` | The layer name from `stratum.config.jsonc`. |
| `m.container` | `int` | Stable numeric container id. |
| `m.stage` | `int` | Purity stage (1–4). |
| `m.dependents` | `array` of `int` | Module ids that depend on this module. |
| `m.metadata` | `map` | Reserved for future `@stratum-*` annotations; currently empty. |

> **Phase 9 limitation:** raw import specifier strings are not yet exposed on `ModuleView`. Rules that need them must wait for the Phase 10 graph augmentation. Use `m.dependents` + graph-shape reasoning until then.

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

Each registered `custom/...` slug consumes one `RuleId` starting from `1000` (see `stratum_plugins_rhai::PLUGIN_ID_FLOOR`). The order in `stratum.config.jsonc` determines the id assignment. Built-in rule ids stay in `1..=9`; the gap reserved for built-ins prevents collisions.
