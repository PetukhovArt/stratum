# Stratum plugin for Claude Code

Wires Stratum into Claude Code: `stratum-lsp` diagnostics after each edit of `.ts/.tsx/.mts/.cts/.js/.vue`, a `stratum-lint hook` gate on Stop, and the `stratum` skill. The contract it builds on is [docs/harness.md](../docs/harness.md).

## Install

1. Put both binaries on `PATH` (check: `which stratum-lint stratum-lsp`, or `Get-Command` in PowerShell):
   - npm (all platforms): `npm install -g @stratum/lint`
   - Windows: `scoop bucket add stratum https://github.com/PetukhovArt/scoop-stratum-bucket` and `scoop install stratum-lint`
   - macOS / Linux: `brew tap petukhovart/stratum` and `brew install stratum-lint`
   - from source: `cargo install --path crates/stratum-lint` and `cargo install --path crates/stratum-lsp`
2. In Claude Code:
   ```
   /plugin marketplace add PetukhovArt/stratum
   /plugin install stratum@stratum
   ```

A binary missing from `PATH` shows as `Executable not found in $PATH` in the `/plugin` Errors tab.

Other harnesses (Codex, Cursor): [docs/agents-snippet.md](../docs/agents-snippet.md).
