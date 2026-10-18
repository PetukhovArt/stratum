Paste into `AGENTS.md` / `CLAUDE.md` of a TS/Vue project for harnesses without the Claude Code plugin (Codex, Cursor). Needs `stratum-lint` and `stratum-lsp` on `PATH`.

```md
## Architecture (Stratum)
- Editor/LSP: launch `stratum-lsp` over stdio for .ts .tsx .mts .cts .js .jsx .mjs .cjs .vue; its diagnostics are architecture violations.
- Before creating, moving or renaming a file, or adding an import across layers or slices: `stratum-lint context <file> --format agent`, then place the file and pick imports from it. Its `may not import` line lists what is closed to this file: go through the target container's entry instead.
- Before finishing a task: `stratum-lint --changed --format agent`. Exit 0 = no new errors (warnings may still be listed); 1 = new error-severity violations; 2 = config or tool error; 3 = no errors, but baseline entries in scope are now fixed (a success; run `stratum-lint baseline --prune` as a separate change).
- Report: line 1 is the summary; then one group per rule slug with a count and example sites. Fix by group.
- Unfamiliar slug only: `stratum-lint explain <slug> --format agent`.
- A violation is a true problem (fix the code) or a wrong rule/config. Structure rules (public-api, sibling-isolation, stage-mixing, container-breadth, slice-fan-in) depend on `layers[].slices` and `structure` in `stratum.config.jsonc`; when `context` misdescribes the folders, the config is wrong. For a wrong rule or config, propose a separate change to `stratum.config.jsonc`, a declarative or `.rhai` rule, or `stratum-lint baseline --prune` after fixes; keep suppressions out of the feature change.
```
