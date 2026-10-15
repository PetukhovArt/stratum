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

## LSP contract

- Root: first `workspaceFolders` entry, else `rootUri`, else `rootPath`, else the process cwd. `file:///c%3A/...` URIs are accepted.
- `initialize` answers at once; the whole-project analysis runs in the background and its result is published for every open document.
- Config: `ROOT/stratum.config.jsonc` if present, else zero-config. Editing it or a plugin script referenced from it re-runs the analysis; a broken config or plugin is reported via `window/showMessage` and the last good diagnostics stay.
- `didOpen` / `didChange` / `didSave` re-run the analysis when the file on disk is newer than the last analysis (the graph is read from disk, so unsaved buffer text is ignored). `didClose` clears the file's diagnostics.
- Diagnostic: `source: "stratum"`, `code` = rule slug as in config (`stratum/no-cross-layer-import`, plugin slugs too), `message` = violation message, then the suggestion on the next line. `relatedInformation` points at the imported module of an import violation, or at up to 10 other members of a cycle; a cycle is reported on each member.
- stdout carries JSON-RPC only; logs go to `window/logMessage` (filter with `RUST_LOG`). The process exits on `exit` (code 0 after `shutdown`, else 1).
- Claude Code routes each file extension to one LSP server, so `stratum-lsp` and a TypeScript server cannot both serve `.ts` there.

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

- `stratum.config.jsonc`: layers, per-rule `severity` / `options`, `overrides` by glob, `extends` (local file or a built-in preset).
- Declarative rules in config for common import policies (no scripting).
- `.rhai` plugin rules with access to modules, imports and options; they show up in `rules` / `explain` like built-ins.
- A JSON schema for the config, so agents can author it with validation.
- Invalid options fail loudly (exit 2) instead of silently falling back to defaults.

Shared seam: `stratum_rules::RuleDoc` + `Rule::doc()` / `RuleRegistry::by_slug` / `by_id`.
Built-ins document themselves in `crates/stratum-rules/src/docs.rs`; plugin rules override `doc()`.
