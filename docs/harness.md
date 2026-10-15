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
stratum-lint explain <rule-slug> [--format agent|json]
stratum-lint context <file> [--format agent|json]
stratum-lint rules [--format agent|json]              # list rules incl. plugins with severity + one-line summary
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

- Rules are named by **slug** (`no-cross-layer-import`), never by numeric id.
- Paths are relative to ROOT with forward slashes.
- The first line is a one-line summary; a clean run prints exactly that one line.
- Repeated violations of one rule collapse into one group line with a count and a few examples.
- `json` output carries `"version"`; breaking changes bump it.

### Baseline

Entries are fingerprinted by `rule slug + relative file + message`, without line numbers, so line shifts do not resurrect them.
A run reports `N baselined` and `M fixed — run "stratum-lint baseline --prune"`; the baseline only ever shrinks unless regenerated on purpose.

## Customization

Everything an architecture rule does is configurable without forking:

- `stratum.config.jsonc`: layers, per-rule `severity` / `options`, `overrides` by glob, `extends` (local file or a built-in preset: `stratum:recommended`, `stratum:fsd`).
- `stratum/import-policy`: declarative import boundaries in config (from / deny / allow / same-slice / message), no scripting.
- `.rhai` plugin rules with access to modules, imports and options; an optional `fn doc()` makes them show up in `rules` / `explain` like built-ins.
- `stratum.schema.json` at the repo root, generated from the config types; configs reference it with `"$schema"`.
- Invalid config fails loudly (exit 2): unknown keys, unknown rule slugs, bad options (the message names slug and key), extends cycles.

Reference: [custom-rules.md](custom-rules.md); a real-world config: [examples/private-project.stratum.config.jsonc](examples/private-project.stratum.config.jsonc).

Shared seam: `stratum_rules::RuleDoc` + `Rule::doc()` / `RuleRegistry::by_slug` / `by_id`.
Built-ins document themselves in `crates/stratum-rules/src/docs.rs`; plugin rules override `doc()`.
