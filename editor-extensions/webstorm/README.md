# Stratum for WebStorm (and other JetBrains IDEs)

Architectural linting in WebStorm / IntelliJ IDEA / PyCharm / RubyMine / etc.
via [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij).

## Install

1. Install Stratum, verify `stratum-lsp --version` works in your shell:

   ```bash
   cargo install --path ../../crates/stratum-lsp
   stratum-lsp --version
   ```

   Or grab the prebuilt binaries from the
   [GitHub release](https://github.com/PetukhovArt/stratum/releases).

2. Install **LSP4IJ** in WebStorm:
   - `File → Settings → Plugins → Marketplace`
   - Search "LSP4IJ" → Install → restart.

3. Import this descriptor:
   - `File → Settings → Languages & Frameworks → Language Servers → +`
   - "Import from JSON"
   - Select `editor-extensions/webstorm/lsp4ij-stratum.json` from this repo.

4. Open a TypeScript / Vue project containing `stratum.toml`. Squiggles appear
   inline as you'd expect from any LSP-backed linter.

## Seeing Stratum's logs (LSP console)

- `View → Tool Windows → LSP Console`
- Filter on "Stratum".
- You see every `window/logMessage` Stratum emits: startup, rule timings,
  errors, file-by-file rule output.

The descriptor sets `"trace": "messages"`, so the console also records every
LSP request/response if you need to debug a hang.

## Troubleshooting

- **"stratum-lsp not found"** — WebStorm uses the GUI shell's PATH, which
  often differs from your terminal's. Either move `stratum-lsp` to a system
  location (`/usr/local/bin`, `C:\Windows\System32`) or hard-code the absolute
  path in `lsp4ij-stratum.json` under `commandLine`.
- **LSP4IJ says "server failed"** — click the server name in the LSP Console
  to see stderr from Stratum. If it crashed during init, the panic message is
  there.
- **No diagnostics on a Vue file** — confirm the file ends in `.vue` and the
  project has `stratum.toml`. The `fileTypeMappings` block is what tells
  LSP4IJ to attach the server to a given file.
