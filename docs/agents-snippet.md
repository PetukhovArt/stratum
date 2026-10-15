Paste into `AGENTS.md` / `CLAUDE.md` of a TS/Vue project for harnesses without the Claude Code plugin (Codex, Cursor). Needs `stratum-lint` and `stratum-lsp` on `PATH`.

```md
## Architecture (Stratum)
- Editor/LSP: launch `stratum-lsp` over stdio for .ts .tsx .mts .cts .js .vue; its diagnostics are architecture violations.
- Before creating, moving or renaming a file, or adding an import across layers: `stratum-lint context <file> --format agent`, then place the file and pick imports from it.
- Before finishing a task: `stratum-lint --changed --format agent`. Exit 0 = done; 1 = violations; 2 = config or tool error.
- Report: line 1 is the summary; then one group per rule slug with a count and example sites. Fix by group.
- Unfamiliar slug only: `stratum-lint explain <slug> --format agent`.
- A violation is a true problem (fix the code) or a wrong rule/config. For a wrong rule, propose a separate change to `stratum.config.jsonc`, a declarative or `.rhai` rule, or `stratum-lint baseline --prune` after fixes; keep suppressions out of the feature change.
```
