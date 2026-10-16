# Stratum plugin for Claude Code

One skill, `stratum`, that tells the agent when to run the `stratum-lint` CLI: `context` before placing a file, `--changed --format agent` before finishing, `explain` for an unfamiliar rule, and a config change or `baseline --prune` when a rule or the layer/slice map is wrong. The contract it builds on is [docs/harness.md](../docs/harness.md).

To enforce the gate rather than leave it to the agent, run `stratum-lint --changed --format agent` from your harness, pre-commit or CI ([docs/ci.md](../docs/ci.md)).

LSP diagnostics live in the separate `stratum-lsp` plugin ([claude-plugin-lsp](../claude-plugin-lsp/README.md)), because Claude Code gives each extension to one language server and `typescript-lsp` usually holds `.ts`.

## Install

1. Put `stratum-lint` on `PATH` (check: `which stratum-lint`, or `Get-Command` in PowerShell):
   - npm (all platforms): `npm install -g @petukhovart/stratum-lint`
   - Windows: `scoop bucket add stratum https://github.com/PetukhovArt/scoop-stratum-bucket` and `scoop install stratum-lint`
   - macOS / Linux: `brew tap petukhovart/stratum` and `brew install stratum-lint`
   - from source: `cargo install --path crates/stratum-lint` (and `cargo install --path crates/stratum-lsp` for the LSP plugin)
2. In Claude Code:
   ```
   /plugin marketplace add PetukhovArt/stratum
   /plugin install stratum@stratum
   ```

Other harnesses (Codex, Cursor): [docs/agents-snippet.md](../docs/agents-snippet.md).
