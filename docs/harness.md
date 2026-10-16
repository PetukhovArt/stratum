# Stratum as a harness tool

Stratum is the **architecture validation** tool of an AI harness: deterministic, fast, token-cheap.
The model brings judgement; Stratum brings facts about the dependency graph.

Integration surfaces, in order of preference:

| Surface | Use | Loop |
|---|---|---|
| `stratum-lint hook` on `PostToolUse` | per-edit check of the edited file in Claude Code (plugin `stratum`), as model context | cold, one-file scope |
| `stratum-lsp` | per-edit diagnostics in any LSP-capable harness (Cursor, Codex…; Claude Code plugin `stratum-lsp` only without `typescript-lsp`) | warm, incremental |
| `stratum-lint --changed --format agent` | end-of-turn / pre-commit / CI gate | cold, scoped report |
| `stratum-lint context <file>` | before editing: where the file sits and what it may import | preventive |
| `stratum-lint explain <rule>` | on demand, only when a rule is unfamiliar | judgement aid |
| `stratum-lint hook` on `Stop` | end-of-turn gate in Claude Code (plugin `stratum`) | gate |
| CI Template (GitLab, GitHub) | MR/PR gate with `--format gitlab` / `github`, `baseline --check` ratchet, advisory `diff --base` report; wiring in [ci.md](ci.md) | cold, whole project |

Source files: `.ts .tsx .mts .cts .js .jsx .mjs .cjs .vue`. No editor extensions are shipped: any LSP client can launch `stratum-lsp` over stdio.

## LSP contract

- `stratum-lsp` takes no arguments and speaks JSON-RPC over stdio.
- Root: first `workspaceFolders` entry, else `rootUri`, else `rootPath`, else the process cwd. `file:///c%3A/...` URIs are accepted.
- `initialize` answers at once; the whole-project analysis runs in the background and its result is published for every open document. Capabilities: open/close + full-text change sync, `hover` (the diagnostic on that line), `documentLink`. No code actions.
- Config: `ROOT/stratum.config.jsonc` if present, else zero-config. Editing it, a config file it `extends`, a plugin script referenced from it, `ROOT/tsconfig.json` or `ROOT/stratum.baseline.json` triggers a full rebuild. A broken config, plugin or baseline is reported via `window/showMessage` (once per distinct error) and the last good diagnostics stay.
- Baseline: violations matched by `ROOT/stratum.baseline.json` are not published, as in the CLI and the hook.
- Unsaved buffers count. The graph stays in memory: `didOpen` / `didChange` re-parse that document from its buffer text, replace its outgoing edges and re-run the rules; nothing re-runs when its imports, purity, exports and tracked calls are unchanged. `didClose` clears the file's diagnostics and reverts it to the text on disk. A full rebuild re-applies every open buffer.
- A full rebuild also runs on a `workspace/didChangeWatchedFiles` create or delete (the server registers a watcher for the source extensions when the client supports dynamic registration), for a document created after the last build, and when an incremental update fails. A watched change to a closed file re-parses it from disk.
- Diagnostics are published only for open documents whose diagnostics changed. Each has `source: "stratum"`, `code` = rule slug (`stratum/no-cross-layer-import`, plugin slugs too), `message` = violation message, then the suggestion on the next line. `relatedInformation` points at the imported module of an import violation, or at up to 10 other members of a multi-module violation such as a cycle, which is reported on each member.
- stdout carries JSON-RPC only; logs go to `window/logMessage` (filter with `RUST_LOG`, default `warn,stratum_lsp=info`). The process exits on `exit`: code 0 after `shutdown`, else 1.
- Claude Code routes each file extension to one LSP server, so `stratum-lsp` and a TypeScript server cannot both serve `.ts` there. Hence the separate `stratum-lsp` plugin, and the per-edit hook in the `stratum` plugin.

## CLI contract (stable, versioned)

```
stratum-lint [lint] [ROOT]                     # ROOT defaults to "."
    [--format terminal|json|sarif|agent|gitlab|github]
                                               # default terminal; agent = compact, token-cheap text;
                                               # gitlab = Code Quality JSON; github = annotations, then agent text
    [--config PATH]                            # default ROOT/stratum.config.jsonc, else zero-config
    [--max N]                                  # agent format: cap printed violations, rest summarised (default 20)
    [--files F...]                             # report only violations touching these files (cwd-relative, else ROOT-relative; missing = exit 2)
    [--changed[=REF]]                          # files changed vs REF (default HEAD) + untracked; the "=" is required
    [--baseline PATH | --no-baseline]          # default: ROOT/stratum.baseline.json if it exists
    [--watch]                                  # re-lint on file changes (200 ms debounce) until interrupted
stratum-lint baseline [ROOT] [--config PATH] [--out PATH] [--prune]
                                               # accept current violations (default ROOT/stratum.baseline.json); --prune only drops fixed ones
stratum-lint baseline [ROOT] [--out PATH] --check[=REF]
                                               # fail on Baseline Growth vs the same file at REF (default HEAD); the "=" is required;
                                               # conflicts with --prune and --config
stratum-lint diff [ROOT] --base REF [--config PATH]
                                               # Drift of the working tree vs REF, as markdown on stdout
stratum-lint explain <SLUG> [ROOT] [--format agent|json] [--config PATH]   # slug with or without "stratum/"
stratum-lint rules [ROOT] [--format agent|json] [--config PATH]           # every rule incl. plugins, effective severity, one-line summary
stratum-lint context <FILE> [--root ROOT] [--format agent|json] [--config PATH]
                                               # ROOT default: nearest ancestor of FILE with stratum.config.jsonc, else package.json, else cwd
stratum-lint hook [--max N]                    # reads a Claude Code hook event from stdin; --max default 10 after an edit, 20 on Stop
stratum-lint init [ROOT]                       # writes ROOT/stratum.config.jsonc from zero-config inference
stratum-lint snapshot [ROOT] [--out PATH] [--config PATH]
                                               # Graph Snapshot v3 JSON, default stratum.snapshot.json; config default ROOT/stratum.config.jsonc.
                                               # { version: 3, project, modules[{id, path, container, layer, purity, zone?, territory}],
                                               #   containers, layers, edges, zones[{id, path, root}], territories[] }; paths ROOT-relative with "/"
stratum-lint visualize [ROOT] [--config PATH] [--no-open]
                                               # serves the Visualizer on 127.0.0.1:<random port> until Ctrl+C;
                                               # prints "stratum-visualizer ready at <url>" to stderr; --no-open skips the browser
```

### Exit codes

| Command | 0 | 1 | 2 |
|---|---|---|---|
| `lint` (default), every `--format` | no error-severity violation after baseline and scope; warnings and info may be listed | at least one error-severity violation | tool failure |
| `lint --watch` | never: it runs until killed | | tool failure on any run |
| `baseline` | file written | | tool failure; `--prune` without an existing baseline |
| `baseline --check` | no entry count grew: equal, shrunk, introduced (absent at REF), or no baseline in the working tree | some entry count grew | tool failure; unknown REF; invalid baseline on either side; `--out` outside ROOT |
| `diff` | no drift | drift (advisory: CI Templates let it pass) | tool failure; unknown REF; a graph fails to build on either side |
| `explain` | printed | | tool failure; unknown slug |
| `rules`, `context` | printed, whatever violations exist | | tool failure; `context` FILE outside ROOT |
| `init` | config written | | tool failure; config already exists |
| `snapshot` | snapshot written | | tool failure |
| `visualize` | server stopped with Ctrl+C | | tool failure |
| `hook` | decision printed, or nothing to say | tool failure (Claude Code's non-blocking error) | never |

Tool failure: bad arguments, an invalid config (see Customization), a plugin that fails to load, an unreadable baseline, a failed `git` call for `--changed`, `--check` or `diff`, a missing `--files` path. The message goes to stderr.
An unknown REF carries the hint `git fetch --depth=1 origin <REF>`: a shallow CI clone lacks the base commit until it is fetched.

### Output rules

- Rules are named by **slug** (`stratum/no-cross-layer-import`; plugins use their config key, conventionally `custom/<name>`), never by numeric id.
- Paths are relative to ROOT with forward slashes, except in `gitlab` and `github`: there they are relative to the git work-tree root (ROOT's path inside the repository prepended), where the CI resolves them. Outside a repository both are the same.
- `json` and `sarif` carry full messages; every JSON shape carries `"version"`, and a breaking change bumps it.

`--format agent`:

- The first line is a one-line summary; a clean run prints only that line, starting `stratum: ok, no new violations`.
- Violations collapse into one group per rule and severity: a header with the count and an `explain` hint (once per rule), up to 3 examples, `... N more`, and the shared `fix:` line when every hit has the same suggestion.
- `-> file` is the import target when the violation has an edge.
- Groups are ordered by severity, then count. Past `--max` printed examples, the remaining groups collapse into one `+N more rules over --max M` line.
- Info-severity rules are advisory: one count line, no examples, not counted against `--max`.
- Messages longer than 240 chars are clipped, with a `(+N chars, full text in --format json)` note.
- A zero-config run appends `| zero-config: layers inferred from src/*, run "stratum-lint init" to pin them` to the summary line. Zero-config makes every `src/*` folder a layer and applies the `stratum:recommended` severities with `stratum/public-api`, `stratum/sibling-isolation` and `stratum/container-breadth` off, since they need declared layers and structure.

### CI formats

Both print only the violations left after baseline and scope.

- `--format gitlab`: a GitLab Code Quality JSON array of `{ description, check_name, fingerprint, severity, location: { path, lines: { begin } } }`. `check_name` is the slug; `error` -> `major`, `warning` -> `minor`, `info` -> `info`. `fingerprint` is 32 hex chars from the baseline fingerprint plus the occurrence index among identical ones in line order: stable across line shifts and releases, so the MR widget can diff it against the target branch.
- `--format github`: `::error file=…,line=…,col=…,title=<slug>::<message>` and `::warning …` workflow commands, at most 10 per severity, picked round-robin across rules (most hits first) so one noisy rule cannot hide the rest; info is not annotated. Then the full `--format agent` text.

### Baseline

Entries are fingerprinted by `rule slug + relative file + message`, without line numbers and with every number in the message masked as `#`, so line shifts and changed metrics do not resurrect them.
A run reports `N baselined` and `M fixed - run "stratum-lint baseline --prune"`; the baseline only ever shrinks unless regenerated on purpose.
Matching runs on the whole project before scoping, so `fixed` is project-wide and `baselined` counts only in-scope hits. An empty scope skips the graph build, so both read 0.
Hook output never shows the `fixed` hint: tightening the baseline is a separate change, not part of the agent's current edit.

Baseline file (`stratum.baseline.json`): `{ "version": 1, "violations": [{ rule, file, message, count? }] }`; `message` is stored masked, `count` is omitted when 1.

`baseline --check[=REF]` is the ratchet: it compares the working-tree baseline (`--out`, else `ROOT/stratum.baseline.json`) with the same ROOT-relative path at REF, reads no config and builds no graph. **Baseline Growth** is any fingerprint whose count rose. A fixed entry does not pay for a new one, so a swap at the same total grows, and so does a file move (the file is part of the fingerprint). On growth it prints `Baseline <file> grew by N since <REF>:`, then `+N <slug> <file> <message>` per grown fingerprint, the first 20, then `... N more`. Otherwise it prints one line: not grown, introduced (absent at REF, so adopting a baseline passes), or nothing to check.

### Drift report

`stratum-lint diff --base REF` compares the architecture of the working tree with REF's commit. Both sides are built as graphs with the working tree's config (`--config`, else `ROOT/stratum.config.jsonc`, else zero-config): no rules, plugins or baseline. REF is read from git; nothing is written into the work tree or `.git`.

- Line 1 is the marker `<!-- stratum:drift -->`, or `<!-- stratum:drift <prefix> -->` when ROOT is the subdirectory `<prefix>` of the repository. CI Templates key their single MR note / PR comment per project root on this exact line.
- Line 2 is `### Stratum: architecture drift vs <short sha>`; line 3 a one-line summary, `No architecture drift.` when there is none.
- Then one `#### <kind> (<count>)` section per non-empty kind: layer edges, new slice edges, cycles, container stage changes, slice fan-in growth. A section lists 10 items, then `- ... N more`.
- The report stops at 8 KiB with a last line telling to run `stratum-lint diff --base <REF>` locally for the full report.
- Drift is advisory: CI Templates treat exit 1 as a warning, never as a failed gate.

### Sample output

From `tests/fixtures/tiny-ts-violations`, `stratum-lint --format agent`, exit 1:

```
stratum: 3 errors, 2 warnings, 1 info, 0.00s
error stratum/purity (2) - stratum-lint explain stratum/purity
  src/features/cart/index.ts:1 -> src/features/cart/cart.tsx  Module at purity 2 imports from purity 3: purer code cannot depend on impurer code
  src/shared/api/http.ts:1 -> src/features/cart/cart.tsx  Module at purity 1 imports from purity 3: purer code cannot depend on impurer code
  fix: Move shared logic into a module at or below the lower purity, or inject the impure dependency at composition root.
error stratum/no-circular-deps (1) - stratum-lint explain stratum/no-circular-deps
  src/features/cart/boom.ts:1  Circular dependency: boom.ts -> circle.ts
  fix: Break the cycle by extracting the shared piece into a lower-purity module or by inverting one of the dependencies.
warning stratum/no-cross-layer-import (1) - stratum-lint explain stratum/no-cross-layer-import
  src/shared/api/http.ts:1 -> src/features/cart/cart.tsx  Module imports from layer 'features' which 'shared' does not depend on
  fix: Move the imported symbol into a layer 'shared' may reach, or add 'features' to 'shared'.depends_on if the dependency is legitimate.
warning stratum/public-api (1) - stratum-lint explain stratum/public-api
  src/shared/api/http.ts:1 -> src/features/cart/cart.tsx  Import of `src/features/cart/cart.tsx` reaches inside `src/features/cart`
  fix: Import from `src/features/cart` through its entry `src/features/cart/index.ts`, exporting the name there if it is missing.
info (advisory, details in --format json): stratum/depth-ratio (1)
```

The same fixture after `stratum-lint baseline`, breaking the cycle and adding a `shared → features` import, with `--files src/shared/api/client.ts`, exit 0 (warnings only):

```
stratum: 2 warnings, 1 info, 1 fixed - run "stratum-lint baseline --prune", scope 1 file, 0.00s
warning stratum/no-cross-layer-import (1) - stratum-lint explain stratum/no-cross-layer-import
  src/shared/api/client.ts:1 -> src/features/cart/boom.ts  Module imports from layer 'features' which 'shared' does not depend on
  ...
```

`--format json` (version 2): `{ "version": 2, "summary": { errors, warnings, infos, baselined, fixed, scope_files, zero_config, elapsed_ms }, "violations": [{ rule, severity, message, file, location: { line, column }, edge: { from, to, kind } | null, suggestion }] }`. `scope_files` is `null` without `--files` / `--changed`; `kind` is `static`, `di` or `runtime`.

`--format sarif` (SARIF 2.1.0) uses slugs as `ruleId` and ROOT-relative `uri` with `uriBaseId: "%SRCROOT%"`.

`rules --format json`: `{ "version": 1, "rules": [{ slug, severity, default_severity, summary }] }`, every registered rule including those that are off. The agent format lists active rules only, after a `N active rules (M off: ...)` line:

```
12 active rules (3 off: stratum/slice-fan-in, stratum/restricted-calls, stratum/stage-mixing)
error   stratum/no-cross-layer-import  A module may import only from its own layer and from the layers listed in its layer's `depends_on`.
...
```

`explain --format json`: `{ "version": 1, "slug", "severity", "default_severity", "doc": { summary, why, fix, options } | null }`. The agent format prints `what:`, `why:`, `fix:` and `options:` lines.

### Context brief

`stratum-lint context` accepts a path that does not exist yet and always exits 0 on success. Agent format, for a file in a `stratum:fsd` project whose store imports a sibling slice:

```
src/features/cart/model/store.ts: layer features, purity 2, visibility public
container src/features/cart (slice, stage 3, entry index.ts): import other containers only via their entry
may import: layers entities, features, shared; modules at purity <= 2
private zones visible here: src/features/cart/_shared
may not import: other slices of src/features/, entry included; files behind another container's entry
imported by: layers app, pages, widgets, features; 1 dependent modules
imports: 2 modules, 2 violate:
  -> src/features/user/api.ts  stratum/public-api (error)
  -> src/features/user/api.ts  stratum/sibling-isolation (error)
violations: 2 (2 error)
```

- A new file gets ` (new file)` after its path and no `imports`/`violations` lines; a file outside every layer gets one line listing the layers.
- `container` is the innermost container; `, in slice ...` follows when that container sits inside a slice.
- `may not import` lists only what active rules enforce: sibling slices and sibling sub-units (`stratum/sibling-isolation`), files behind another container's entry (`stratum/public-api`).
- Lists of imports and other violations show the first 2.

`--format json` (version 2): `{ version, file, status: module|new_file|not_a_source_module|outside_layers, layer, container: { path, kind: layer|slice|sub_unit|nested, stage, entry } | null, slice (same shape) | null, purity, visibility: "public" | "private to <dir>", may_import_layers, max_import_purity, private_zones, may_not_import: [{ kind: sibling_slices|sibling_sub_units, parent } | { kind: container_internals }], imported_by_layers, imports, violating_imports: [{ target, rule, severity }], dependents, violations: [{ rule, severity, message, on_import }], layers: ["<id>=<path>"] }`.

### Hook

`stratum-lint hook` reads one Claude Code hook event from stdin:

- `Stop` / `SubagentStop` with `stop_hook_active: false`: runs `--changed --format agent` in the event's `cwd` and prints `{"decision":"block","reason":"<report>"}` when new error-severity violations remain.
- `PostToolUse` of `Edit` / `Write` for a source file under `cwd`: lints with that file as scope (baseline honoured), drops info findings, and when error or warning violations remain prints `{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"<report>"}}`. From the fixture above, after an edit of `src/shared/api/http.ts`, the report starts with `stratum: 1 error, 2 warnings, scope 1 file, 0.00s`.
- Everything else exits 0 silently. A tool failure exits 1, not 2, because exit 2 would block the stop with the error text.

## Customization

Everything an architecture rule does is configurable without forking: layers and slices, `structure` folder names, per-rule `severity` / `options`, `overrides` by glob (severity only), `project.ignore`, `extends` (a local file or the presets `stratum:recommended`, `stratum:fsd`), the declarative `stratum/import-policy` and `stratum/restricted-calls` rules, and `.rhai` plugin rules with an optional `fn doc()` that makes them show up in `rules` / `explain` like built-ins. Configs reference `stratum.schema.json` (repo root, generated from the config types) with `"$schema"`.

Invalid config fails loudly, with exit 2 in every command that loads it (exit 1 in `hook`) and `window/showMessage` in the LSP: unknown keys (`project.root` included), unknown rule slugs, bad options (the message names slug and key), a `depends_on` naming an undeclared layer, bad `ignore` / `overrides[].files` globs, `options` or `script` in an override, an unknown preset, extends cycles.

Reference: [custom-rules.md](custom-rules.md); a real-world config: [examples/private-project.stratum.config.jsonc](examples/private-project.stratum.config.jsonc).
Built-in rule docs (`explain`, `rules`) live in `crates/stratum-rules/src/docs.rs`.
