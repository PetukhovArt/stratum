---
name: stratum
description: Layer-aware edits in TS/Vue projects checked by Stratum (stratum-lint, stratum-lsp). Use before creating, moving or renaming a source file or adding an import across layers or slices; before finishing a change to source files; when a Stratum report or diagnostic names a rule slug; when a Stratum rule looks wrong for the project.
---

# Stratum

Stratum supplies facts about the dependency graph; you supply the judgement. Rules are named by **slug**; paths are relative to the project root.

## Before the edit

Run `stratum-lint context <file> --format agent` before you create, move or rename a file, or add an import that crosses a layer or slice. For a new file, pass the path it will have. The output says where the file sits and what it may import: place the file and pick imports from it. Its `may not import` line names what is closed to this file (sibling slices, sibling sub-units, files behind another container's entry): import through the target container's entry instead. `context` is read-only and accepts a path that does not exist yet: run it whenever you decide where a module goes, even when you only answer a question and create nothing.

## Before finishing

Run `stratum-lint [ROOT] --changed --format agent` once the source edits of the change are done: it reports the new violations in the files changed since `HEAD`, untracked included. The change is finished at exit 0; on exit 1, fix and rerun. For one file mid-change, `--files <file>` gives the same report scoped to it.

## Reading a report

- Line 1 is the summary. A clean run is that one line: done.
- Each group is one rule slug with a count and a few example `file:line` sites. Fix by group: the examples show the pattern, then find every site of it.
- `N baselined` is accepted legacy debt, outside this change. `M fixed - run "stratum-lint baseline --prune"` means your work shrank the debt.
- Exit 0 clean, 1 error-severity violations remain, 2 tool or config failure: read the message and fix the config or arguments.

Run `stratum-lint explain <slug> --format agent` only for a slug you have not met in this session. `stratum-lint rules --format agent` lists every rule, plugin rules included.

## Judgement

Each violation is one of two things:

1. **True problem**: the code breaks the architecture. Move the code, invert the dependency, or go through the entry of the slice. This is the usual case.
2. **Wrong rule or config**: the layer map, an option or the rule itself misdescribes how this project is built. Evidence: the "fix" makes the design worse, or the same pattern is deliberate across many files.

The structure rules (`public-api`, `sibling-isolation`, `stage-mixing`, `container-breadth`, `slice-fan-in`) see containers that Stratum derives from `layers[].slices` and `structure` (entries, segments, private zones). When `context` shows a container, kind or entry that does not match how the folders are really used, the config is wrong, not the code: propose the `slices` / `structure` fix as a config change.

When unsure, treat it as a true problem and say why in your reply.

## Improving the rules

A wrong rule is fixed at its source, so the next run of every agent and teammate benefits. Keep the feature change free of rule changes and propose the rule change on its own:

- Config: layers and `slices`, `structure`, `severity`, `options` or `overrides` by glob (severity only) in `stratum.config.jsonc` (a JSON schema validates it).
- A missing policy: `stratum/import-policy` or `stratum/restricted-calls` in config, or a `.rhai` plugin rule when declarations cannot express it.
- Debt you paid off: `stratum-lint baseline --prune` (only drops fixed entries).

Write the proposal as a separate commit or PR with the violation, why the rule is wrong here, and the diff. Show it to the user before applying it. Inline suppression and baseline regeneration hide the signal: reserve them for an explicit user request.
