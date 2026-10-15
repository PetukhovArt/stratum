# Stratum LSP plugin for Claude Code

Registers `stratum-lsp` as the language server for `.ts/.tsx/.mts/.cts/.js/.vue`. Claude Code pushes its diagnostics into context after each edit.

## When to enable

Claude Code gives each file extension to exactly one LSP server, the first one registered ([LSP servers](https://code.claude.com/docs/en/plugins/components#lsp-servers)). With the official `typescript-lsp` plugin enabled, this plugin collides with it on `.ts`:

- Project with `typescript-lsp`: skip this plugin. The `stratum` plugin already lints each edited file through its `PostToolUse` hook.
- Project without `typescript-lsp`: enable it for warm, incremental diagnostics, info severity included. Next to the `stratum` plugin, an edited file with errors or warnings is then reported twice: by this server and by the hook.

The per-server `diagnostics` field (default `true`) controls whether Claude Code pushes this server's diagnostics into context after edits ([manifest reference](https://code.claude.com/docs/en/plugins-reference#lspservers)). Setting it to `false` keeps the server for hover and document links only and leaves the reporting to the hook.

## Install

`stratum-lsp` on `PATH` (see [the stratum plugin README](../claude-plugin/README.md#install)), then:

```
/plugin marketplace add PetukhovArt/stratum
/plugin install stratum-lsp@stratum
```
