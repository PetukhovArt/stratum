# Stratum as a harness tool

Stratum is the **architecture validation** tool of an AI harness: deterministic, fast, token-cheap.
The model brings judgement; Stratum brings facts about the dependency graph.

Integration surfaces, in order of preference:

| Surface | Use | Loop |
|---|---|---|
| `stratum-lsp` | per-edit diagnostics in any LSP-capable harness (Claude Code plugin `lspServers`, Cursor, Codex…) | warm, incremental |
| `stratum-lint --changed --format agent` | end-of-turn / pre-commit / CI gate | cold, scoped report |
| `stratum-lint context <file>` | before editing: where the file sits and what it may import | preventive |
| `stratum-lint explain <rule>` | on demand, only when a rule is unfamiliar | judgement aid |
| `stratum-lint hook` | thin adapter for Claude Code hook events (stdin JSON) | gate |

No editor extensions are shipped: any LSP client can launch `stratum-lsp` over stdio.

## CLI contract (stable, versioned)

```
stratum-lint [lint] [ROOT]
    [--format terminal|json|sarif|agent]   # agent = compact, token-cheap text
    [--config PATH]
    [--max N]                              # cap printed items; rest summarised (agent default 20)
    [--files F...]                         # report only violations touching these files
    [--changed[=REF]]                      # files changed vs REF (default HEAD) + untracked
    [--baseline PATH | --no-baseline]      # default: ROOT/stratum.baseline.json if it exists
    [--watch]
stratum-lint baseline [ROOT] [--out PATH] [--prune]   # accept current violations; --prune only drops fixed ones (ratchet)
stratum-lint explain <rule-slug> [ROOT] [--format agent|json] [--config PATH]   # slug with or without "stratum/"
stratum-lint context <file> [--root ROOT] [--format agent|json] [--config PATH]
stratum-lint rules [ROOT] [--format agent|json] [--config PATH]   # active rules incl. plugins with severity + one-line summary
stratum-lint hook                                      # reads a Claude Code hook event from stdin
stratum-lint init | snapshot | diff | visualize        # unchanged
```

### Exit codes

| Code | Meaning |
|---|---|
| 0 | no error-severity violations (after baseline and scope) |
| 1 | at least one error-severity violation |
| 2 | tool failure: bad config, bad args, plugin load error |

### Output rules (all agent-facing formats)

- Rules are named by **slug** (`stratum/no-cross-layer-import`; plugins `custom/<name>`), never by numeric id.
- Paths are relative to ROOT with forward slashes.
- The first line is a one-line summary; a clean run prints exactly that one line.
- Repeated violations of one rule collapse into one group line with a count and a few examples.
- `json` output carries `"version"`; breaking changes bump it.

### Baseline

Entries are fingerprinted by `rule slug + relative file + message`, without line numbers, so line shifts do not resurrect them.
A run reports `N baselined` and `M fixed — run "stratum-lint baseline --prune"`; the baseline only ever shrinks unless regenerated on purpose.

### Gate output

`--format agent`:

```
stratum: 1 error, 2 warnings, 4 baselined, 1 fixed - run "stratum-lint baseline --prune", scope 3 files, 0.42s
error stratum/no-circular-deps (1) - stratum-lint explain stratum/no-circular-deps
  src/a.ts:1  Circular dependency: a.ts -> b.ts
  fix: Break the cycle ...
warning stratum/no-cross-layer-import (2) - stratum-lint explain stratum/no-cross-layer-import
  src/shared/x.ts:1 -> src/features/y.ts  Module imports from layer 'features' ...
  ...
+2 more rules over --max 20: stratum/depth-ratio (3), ...
```

- `-> file` is the import target when the violation has an edge.
- Messages longer than 240 chars are clipped; `--format json` has the full text.
- A zero-config run appends `| zero-config: layers inferred from src/*, ...` to the summary line.

`--format json` (version 2): `{ "version": 2, "summary": { errors, warnings, infos, baselined, fixed, scope_files, zero_config, elapsed_ms }, "violations": [{ rule, severity, message, file, location: { line, column }, edge: { from, to, kind } | null, suggestion }] }`.
`--format sarif` uses slugs as `ruleId` and ROOT-relative `uri` with `uriBaseId: "%SRCROOT%"`.

Baseline file (`stratum.baseline.json`): `{ "version": 1, "violations": [{ rule, file, message, count? }] }`; `count` is omitted when 1.
Matching runs on the whole project before scoping, so `fixed` is project-wide and `baselined` counts only in-scope hits. An empty scope skips the graph build, so both read 0.

`stratum-lint hook`: on `Stop` / `SubagentStop` with `stop_hook_active: false` it runs `--changed --format agent` in the event's `cwd` and prints `{"decision":"block","reason":"<report>"}` when new error-severity violations remain.
Everything else exits 0 silently. A tool failure inside the hook exits 1 (Claude Code's non-blocking error), not 2, because exit 2 would block the stop with the error text.

## Customization

Everything an architecture rule does is configurable without forking:

- `stratum.config.jsonc`: layers, per-rule `severity` / `options`, `overrides` by glob, `extends` (local file or a built-in preset).
- Declarative rules in config for common import policies (no scripting).
- `.rhai` plugin rules with access to modules, imports and options; they show up in `rules` / `explain` like built-ins.
- A JSON schema for the config, so agents can author it with validation.
- Invalid options fail loudly (exit 2) instead of silently falling back to defaults.

Shared seam: `stratum_rules::RuleDoc` + `Rule::doc()` / `RuleRegistry::by_slug` / `by_id`.
Built-ins document themselves in `crates/stratum-rules/src/docs.rs`; plugin rules override `doc()`.
