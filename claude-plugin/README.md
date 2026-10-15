# Stratum plugin for Claude Code

Wires Stratum into Claude Code. The contract it builds on is [docs/harness.md](../docs/harness.md).

- After each `Edit` / `Write` of a `.ts/.tsx/.mts/.cts/.js/.jsx/.mjs/.cjs/.vue` file under the project, `stratum-lint hook` lints that file and adds its new error and warning violations to the model's context (capped at 10 examples). A clean file adds nothing.
- On Stop, the same command gates the changed files and blocks the stop while new error-severity violations remain.
- The `stratum` skill: run `stratum-lint context` before placing a file, read reports by rule group, and propose a config change when a rule or the layer/slice map is wrong.

LSP diagnostics live in the separate `stratum-lsp` plugin ([claude-plugin-lsp](../claude-plugin-lsp/README.md)), because Claude Code gives each extension to one language server and `typescript-lsp` usually holds `.ts`.

## Install

1. Put `stratum-lint` on `PATH` (check: `which stratum-lint`, or `Get-Command` in PowerShell):
   - npm (all platforms): `npm install -g @stratum/lint`
   - Windows: `scoop bucket add stratum https://github.com/PetukhovArt/scoop-stratum-bucket` and `scoop install stratum-lint`
   - macOS / Linux: `brew tap petukhovart/stratum` and `brew install stratum-lint`
   - from source: `cargo install --path crates/stratum-lint` (and `cargo install --path crates/stratum-lsp` for the LSP plugin)
2. In Claude Code:
   ```
   /plugin marketplace add PetukhovArt/stratum
   /plugin install stratum@stratum
   ```

Other harnesses (Codex, Cursor): [docs/agents-snippet.md](../docs/agents-snippet.md).
