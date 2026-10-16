# Ubiquitous Language

## Graph structure

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Compound DAG** | The in-memory model: a nesting tree of **Containers**, a dependency graph of **Edges**, and **Visibility Scopes** over the same set of **Modules**. | compound graph, architecture graph, dep graph |
| **Module** | One source file (`.ts .tsx .mts .cts .js .jsx .mjs .cjs .vue`) that is a node in the **Compound DAG**. Carries `id`, `path`, `container` (the innermost **Container** that encloses it), `layer`, `purity`, `visibility`, `exports`, `calls`. Not folder-structure's "module": that one is a **Container**. | file, component, unit |
| **Exports** | The names a **Module** exposes to importers: distinct top-level export names (types included; overloads and merged declarations once; `export default` / `export =` as `default`) plus 1 per opaque `export * from`. CommonJS `module.exports` is not counted. For an **SFC**: the plain `<script>` exports plus the `<script setup>` macro members (see **Vue MVP Scope**). Read by `deep-module` and **Depth Ratio**. | public API, export count, surface |
| **Import** | An `import`, `export … from`, `require()` or dynamic `import()` of another file in a **Module**'s source. The parser records it as a **Raw Import**; resolved to a **Module** inside a **Layer**, it becomes an **Edge**. | dependency, link, reference |
| **Raw Import** | An **Import** as the parser emits it, before resolution: literal `specifier`, byte **Source Span**, **Edge** kind and a `type_only` flag. | unresolved import, pre-edge |
| **Edge** | A resolved, typed dependency from one **Module** to another, made from an **Import**. Kinds: `static`, `di`, `runtime`. | arrow, line, connection |
| **Static edge** | An **Edge** from a compile-time `import`, an `export … from` or a `require()`. | static import |
| **DI edge** | An **Edge** wired through a composition root or DI container. No parser emits it yet. | injection, wiring |
| **Runtime edge** | An **Edge** established at runtime. Emitted today for dynamic `import()`. | event link, dynamic dep |
| **Type-only Import** | A **Raw Import** marked `type_only` (`import type { X } from '...'`). It still becomes an ordinary **Edge**; no rule distinguishes it yet. | erased import, types-only |
| **Cycle** | A strongly-connected component in the dependency graph, found via Tarjan SCC. | loop, recursion |
| **Graph Snapshot** | The versioned, deterministic JSON shape of the **Compound DAG**: `version`, `project`, `modules`, `containers`, `layers`, `edges`, `zones` (the **Private Zones**) and `territories` (each distinct **Territory** once; a module points at its own by index). Paths are root-relative. Written by `stratum-lint snapshot`, served by `stratum-lint visualize` to the **Visualizer**. Version 3; a bump needs a coordinated frontend release. | snapshot file, dump, graph file |
| **Source Location** | A 1-based `line:column` pointer carried inside a **Violation**. Distinct from a **Source Span**. | position, line/col |
| **Source Span** | A byte-offset half-open range `[start, end)` into source text, used by the parser to point at AST fragments. | byte range, AST span |
| **`@stratum-purity` Annotation** | A `// @stratum-purity N` (`N ∈ 1..=4`) directive in the leading comments of a source file. It sets the **Module**'s **Purity**; without it the purity is 2. There is no annotation for **Stage**. | stage directive, purity hint |
| **SFC** | Vue Single-File Component (a `.vue` file): optional `<template>`, `<script>` and/or `<script setup>`, and `<style>` blocks. | Vue SFC, .vue file |
| **Vue MVP Scope** | Only `<script>` and `<script setup>` blocks are parsed; no template parsing, no macro expansion. For **Exports** only, `<script setup>` macros are read: each `defineProps` / `defineEmits` member, each `defineExpose` property, each `defineModel` call. Mixed `<script>` + `<script setup>` merge into one **Module** per file. | Vue MVP, SFC MVP scope |

## Methodology

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Methodology** | The configurable contract the tool enforces: **Layers**, the folder names in `structure` (**Entries**, **Segments**, **Private Zones**), **Purity**, **Visibility Scopes** and **Rules**. Names live only in a **Preset** or the user's **Config**, never in code. | architecture style, convention, paradigm |
| **Stratum** | The tool. It knows only the **Methodology** contract; FSD is one **Preset**. | Stratum architecture, the methodology |
| **Preset** | A built-in **Config** to `extends`: `stratum:recommended` sets severities, `stratum:fsd` extends it with the Feature-Sliced Design **Layers**, **Slices**, **Segments** and stricter severities. | methodology bundle, profile, ruleset |
| **Layer** | A named folder of **Modules** with a `depends_on` list of the layers it may import, and `slices: true` when its direct children are **Slices**. A file belongs to the layer with the longest matching path; a file outside every layer is not in the **Compound DAG**. | tier, level |
| **Purity** | A rank from 1 (most pure) to 4 (side effects). A **Module** at purity `s` may import another at purity `t` only when `t <= s`. Set by `@stratum-purity`, default 2. | stage, rank |
| **Visibility Scope** | Who may import a **Module**: `Public`, or `Private { root }` for a module inside a **Private Zone**, importable only from under `root` (the zone's parent folder; the innermost zone wins). | access scope, boundary |
| **Deep Module** | A **Module** whose **Exports** are few relative to what it does internally. The `deep-module` rule caps the export count at `max_exports`. | thick module, encapsulated module |

## Structure

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Container** | A unit of structure: a **Layer** root, a **Slice**, a **Sub-unit** or a nested container (an entry-bearing folder under a **Segment** or **Collection Folder**, or directly under a layer without slices). Has a `path`, a `kind`, a **Stage** and an optional **Entry**; its `parent` chain ends at the layer's container. Derived from the folder tree, never declared. folder-structure calls it a "module". | unit, box, group |
| **Stage** | The folder-structure evolution stage of a **Container**: 1 a single file, 2 a flat folder, 3 a folder split into **Segments**, 4 a folder composed of **Sub-units**. Computed from the folder tree, never annotated. Not **Purity**. | phase, level |
| **Slice** | A **Container** at the first level of a **Layer** declared `slices: true`: each direct child folder or source file, except the layer's own **Entry**. A file slice has **Stage** 1 and is its own **Entry**. In FSD, `features/cart` is a slice. | feature, package |
| **Sub-unit** | An entry-bearing folder directly under a **Slice**, **Sub-unit** or nested **Container**. Isolated from its siblings; it makes its parent **Stage** 4. | sub-feature, submodule |
| **Segment** | A folder named in `structure.segments` that groups a **Container**'s files by purpose (FSD: `model`, `ui`, `api`). Never a **Container**, even with an **Entry** file; it makes its parent **Stage** 3. | layer folder |
| **Entry** | The file named in `structure.entries` (default `index.ts` and its siblings) that is a **Container**'s only import target from outside. A folder that holds one is a **Container**. | barrel, public API file |
| **Private Zone** | A folder named in `structure.private` (default `_shared`), visible only inside its parent folder's subtree. Never a **Container**; counts toward no **Stage**. | local shared |
| **Collection Folder** | A folder with no **Entry**. Its files belong to the enclosing **Container**. | group folder |

## Metrics

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Metric** | A number computed over the **Compound DAG**. | indicator, stat |
| **Miller Fan-in** | The count of distinct importers of a **Module** from outside its **Slice** (else its **Layer**) whose layer is its own or lists it in `depends_on` (declared importers). Imports between **Sub-units** of one slice do not count. Together with **Promotion Pressure** it partitions the outside importers, so no importer counts twice. | Miller fan-out, cross-container fanout |
| **Miller Limit** | The cap (`max_children`) on a **Module**'s **Miller Fan-in** before it counts as a change hotspot (`miller-limit`). | cognitive limit, fan-out cap |
| **Promotion Pressure** | The count of distinct importers of a **Module** from outside its **Slice** (else its **Layer**) whose layer does not list the module's layer in `depends_on`: the imports a move to a lower layer would make legal (`promotion-pressure`). In **Zero-Config Mode** no layer declares `depends_on`, so every outside importer counts here. | promotion score, promotion metric |
| **Depth Ratio** | The number of **Modules** in a module's **Slice** (else its **Layer**), nested containers included, divided by its **Exports** count; undefined when it exports nothing. Low means the module exposes much relative to what its container hides (`depth-ratio`). | encapsulation ratio |
| **Slice Fan-in** | The count of distinct other **Slices** that import a **Slice**. The `slice-fan-in` rule caps it at `max_fan_in`. | cross-entity pattern, entity fan-out |

## Rules and diagnostics

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Rule** | A check that runs over the **Compound DAG** and emits zero or more **Violations**. | check, lint rule, validator |
| **Built-in Rule** | A **Rule** that ships inside the tool. | native rule, internal rule |
| **Custom Rule** | A user-written **Rule**: a **Rhai Script** registered in the **Config** under a **Rule Slug** with `script`. | user rule, extension |
| **Rule Slug** | The public name of a **Rule**: `stratum/<name>` for built-ins, the config key (conventionally `custom/<name>`) for **Custom Rules**. Every output, the **Baseline** and the **Config** use it; numeric ids never leave the tool. | rule id |
| **Rule Doc** | What a **Rule** checks, why, how to fix a hit, and its options, as `explain` and `rules` show it. A **Custom Rule** supplies one through `fn doc()`. | rule help, rule description |
| **Rule Scope** | The unit a **Rule** checks per call: a **Module**, a **Container**, or the whole project. | rule input, scope kind |
| **Import Policy** | The declarative `stratum/import-policy` **Rule**: config policies of `from` / `except_from` / `deny` / `allow` / `slice` globs that deny **Edges** without scripting. | boundary rule, import rule |
| **Violation** | One reported **Rule** infraction: **Rule Slug**, **Severity**, message, file, **Source Location**, the **Modules** involved, an optional **Edge**, an optional **Suggestion**. | error, issue |
| **Severity** | One of `error`, `warning`, `info`, `off`. Only `error` makes the **Gate** fail. The configured severity is a cap: a **Rule** may report lower, never higher. | level, priority |
| **Advisory** | An `info`-severity **Violation**. The agent format shows advisories as one count line, not counted against `--max`. | info finding, hint |
| **Suggestion** | Human-readable advice attached to a **Violation**. Never an auto-applied fix: a **Violation** has no edit object. | autofix, quick fix, suggested fix |
| **Config** | `stratum.config.jsonc` at the project root: `project` (`extends`, `ignore`), `structure`, `layers`, `rules`, `overrides`. Validated strictly: an unknown key, slug or option is an error. | settings, configuration |
| **Override** | A glob-scoped block in the **Config** (`overrides[]`) that changes a **Rule**'s **Severity** for matching files, and nothing else. The last matching block wins per rule. | exception, scoped rule, override block |
| **Ignore** | `project.ignore` globs, on top of built-in defaults (`node_modules`, `dist`, `out`, `build`, `coverage`, `.git`, `*.d.ts`), that keep files out of the **Compound DAG** entirely. Unlike an **Override**, an ignored file makes no **Edges**. | exclude |
| **Zero-Config Mode** | The run without a **Config**: every folder under `<root>/src` becomes a **Layer** (no `depends_on`, no slices) and the `stratum:recommended` severities apply, except that `public-api`, `sibling-isolation` and `container-breadth` are off until layers and structure are declared. **Private Zones** are still detected by name. | implicit config, auto-config |
| **Baseline** | Accepted **Violations** in `stratum.baseline.json`, matched by fingerprint (**Rule Slug**, file, message with numbers masked). Matched violations are not reported; `baseline --prune` only drops fixed entries, so the baseline only shrinks. | allowlist, suppression file |
| **Baseline Growth** | A rise in some **Baseline** entry's count versus the baseline committed at a git ref: what `baseline --check` fails on. Counted per fingerprint, so a fixed **Violation** does not pay for a new one, and a moved file grows. A baseline absent at the ref is introduced, not grown. | baseline regression, new debt |
| **Drift** | The change in architecture between a git ref and the working tree, as `stratum-lint diff --base` reports it: imports that break rules, layer and **Slice** edges, **Cycles**, **Stage** changes, **Slice Fan-in** growth. Advisory: it never fails a **Gate**. Not a **Violation**: it compares two graphs, and rules judge only the imports present on one side. | architecture diff, regression |
| **Report Scope** | The files a report is limited to: `--files` or `--changed`. A **Violation** is in scope when its file, either end of its **Edge** or any of its **Modules** is. | scope (unqualified) |
| **Gate** | The check that fails on new error-severity **Violations**, run by the caller: an agent harness or pre-commit on the changed files (`stratum-lint --changed --format agent`), CI on the whole project (a **CI Template**). | check run |
| **Brief** | What `stratum-lint context` prints for one file, existing or planned: its **Layer**, **Purity**, **Container** with **Stage** and **Entry**, visible **Private Zones**, what it may and may not import, and its current **Violations**. | context dump, file report |
| **Territory** | What one file, existing or planned, may import and who may import it: the **Layers** it may import and is imported by, the highest **Purity** it may import, the **Private Zones** it sees, and the regions a **Rule** forbids (sibling **Slices** or **Sub-units**, files behind another **Container**'s **Entry**). Derived from the **Config** with each file's **Overrides** applied; a rule that is off for the file forbids nothing. The **Brief** and the **Graph Snapshot** both carry it. | permissions, import scope, allowed set |

## Tooling components

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Stratum Tooling** | The whole product: **Linter**, **LSP Server**, **Visualizer**, **Harness Plugins**, **CI Templates**, and the shared Rust core. | the toolchain, the suite |
| **Linter** | The CLI binary `stratum-lint`: lint, **Gate**, **Baseline**, **Brief**, `explain`, `rules`, `init`, `snapshot`, `visualize`. | scanner, checker |
| **LSP Server** | `stratum-lsp`, the stdio language server AI harnesses launch for per-edit **Violations**. Analysis runs in the background after `initialize`; the graph stays in memory and each open document is re-parsed from its unsaved buffer on change; **Config**, **Baseline**, plugin or `tsconfig.json` changes trigger a full rebuild. No code actions. | language server |
| **Rule Engine** | The `stratum-engine` crate shared by the **Linter** and the **LSP Server**: loads the **Config**, builds the **Compound DAG**, runs the **Rules**, names **Violations** by **Rule Slug** with root-relative paths. | rule runner |
| **Harness Plugin** | A Claude Code plugin that wires Stratum in: `stratum` (a skill) and `stratum-lsp` (the **LSP Server**, only for projects without `typescript-lsp`). Not a **Plugin**. | extension, editor extension |
| **CI Template** | The CI-side adapter for one platform: `ci/gitlab/templates/stratum.yml` (GitLab `include`) or `ci/github` (GitHub composite action). It installs the **Linter** from the job's npm registry and runs the **Gate** in the platform's format, the **Baseline Growth** check and the **Drift** report. All platform knowledge lives here; the binary reads no CI environment. Not a **Harness Plugin**: it wires Stratum into a pipeline, not into an agent. | CI component, CI plugin, pipeline |
| **Visualizer** | The browser UI for the **Compound DAG** and its **Violations**: `stratum-lint visualize` serves the **Graph Snapshot** at `/api/snapshot`, the violations at `/api/violations`, a file's **Brief** at `/api/context?path=`, with `--base REF` the **Drift** at `/api/drift` (404 without it), and the bundled SolidJS frontend, which shows them as SVG views: Module focus, Matrix, Merge request (with the **Drift**), Container map, Module set and Bench. | viewer, graph UI |
| **Watch Mode** | `stratum-lint --watch`: re-runs the lint on any file event under the project root, with a 200 ms debounce. | live mode |
| **Language Extractor** | One parser per language (`LanguageExtractor`): it turns a source file into **Extracted Data**. One handles `.ts .mts .cts .tsx .js .jsx .mjs .cjs`, one handles `.vue`. | parser adapter, frontend |
| **Extracted Data** | What a **Language Extractor** produces for one file: **Raw Imports**, the **Purity** annotation, **Exports**, and calls to tracked names. | parser result, extraction output |
| **Path Resolver** | Resolves a **Raw Import**'s specifier from one file's directory to an absolute path, honouring `tsconfig.json` `paths` aliases. | module resolver, tsconfig resolver |

## Extensibility

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Plugin** | A user extension that delivers a **Custom Rule**: a **Rhai Script**. | extension, addon |
| **Rhai Script** | A **Plugin** written in the Rhai DSL: `fn check(m)` or `fn check(m, options)` returning an array of maps, plus an optional `fn doc()`. No build step; the tool compiles it at load. | dsl rule, scripted rule |
| **Plugin API** | What a **Rhai Script** sees and returns: a **Module View**, the rule's `options`, and an array of maps, each one **Violation** (`message`, optional `line`, optional `suggestion`). The engine is sandboxed: no filesystem, network or process; `eval` disabled; bounded recursion and operations. | extension API, SDK |
| **Module View** | The read-only projection of a **Module** a **Rhai Script** sees: `id`, `path` (absolute, forward slashes), `layer`, `container`, `purity`, `dependents`, `imports` (`#{ path, layer, kind }` per **Edge**), `metadata` (reserved). Raw import specifiers are not exposed. | rhai module, plugin module |
| **Plugin Hot-Reload** | Picking up a changed **Rhai Script** without restarting: the **LSP Server** watches every script referenced from the **Config** and rebuilds; **Watch Mode** re-runs on the next file event. | rhai reload, live plugin |

## Visualization

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Fixture Project** | A reference project under `tests/fixtures/` used as input for tests and benchmarks (e.g. `tiny-ts`, `tiny-ts-violations`). | sample project, test repo |
| **Exemplar App Fixture** | `tests/fixtures/exemplar-stratum-app/`: a curated FSD-shaped **Fixture Project** with deliberate violations; its per-rule violation histogram is a regression snapshot. | exemplar fixture, reference app |

## Relationships

- A **Module** belongs to exactly one **Layer**, sits in exactly one innermost **Container**, and has exactly one **Purity** and one **Visibility Scope**.
- A **Container** has exactly one **Stage**, at most one **Entry**, and a parent chain that ends at its **Layer**'s container. **Segments**, **Private Zones** and **Collection Folders** are never **Containers**.
- Placement is a function of the path set: adding or removing an **Entry** file turns a **Collection Folder** into a **Container** or back.
- An **Edge** has exactly one kind: `static`, `di`, or `runtime`.
- A **Raw Import** is the parser-side precursor to an **Edge**: it carries a textual `specifier` and a **Source Span**, but no **Module**. The **Path Resolver** resolves it; an **Import** of a package or of an ignored or layer-less file makes no **Edge**.
- A **Module** at **Purity** `s` may depend on one at **Purity** `t` iff `t ≤ s`: purer code cannot reach impurer code.
- From outside a **Container**, only its **Entry** may be imported (`public-api`); sibling **Slices** and sibling **Sub-units** may not import each other at all (`sibling-isolation`).
- A **Rule** reads the **Compound DAG** and emits zero or more **Violations**; a **Rule** operates over exactly one **Rule Scope**.
- A **Violation** has one **Severity**, one **Source Location**, and points at one or more **Modules** (and optionally one **Edge**).
- The **Linter** and the **LSP Server** share the **Violation** type through the **Rule Engine**; the **Visualizer** gets violations at `/api/violations` next to the **Graph Snapshot**.
- A **Rhai Script** registers a **Custom Rule** through the **Plugin API**.
- A **Language Extractor** consumes raw source text and produces **Extracted Data**.

## Example dialogue

> **Dev:** "I'm adding `.vue` SFC support. Do I write a new **Language Extractor** or extend the TS one?"
> **Architect:** "New one. **Language Extractor** is a trait per language; the TS extractor handles the TS/JS family. Write the Vue one: it handles `.vue`, and produces **Extracted Data** with the same **Raw Import** shape."
> **Dev:** "Does it resolve specifiers to file paths?"
> **Architect:** "No. A **Language Extractor** only emits **Raw Imports**: literal `specifier` strings and **Source Spans**. The **Path Resolver** turns `'~shared/api/http'` into `/abs/path/src/shared/api/http.ts`; the graph builder, not the parser, makes the **Edge**."
> **Dev:** "What about `import type {...}`? Throw it away?"
> **Architect:** "No. It's a **Type-only Import**, recorded with `type_only: true`, and it becomes an ordinary **Edge**: exposing a type across a forbidden boundary is still a real violation."

## Flagged ambiguities

- **"graph"** means the **Compound DAG** in memory and the **Graph Snapshot** on disk or on the wire. Say which.
- **"import"**: the statement in source is an **Import**, its parsed form a **Raw Import**, its resolved form an **Edge**. User-facing names keep the word "import" for **Edges** (`no-cross-layer-import`, `import-policy`, Rhai `m.imports`, "imports: N modules" in a **Brief**); in prose about the graph say **Edge**.
- **"span"** vs **"location"**: a **Source Span** is a byte range used inside the parser; a **Source Location** is a 1-based `line:column` pointer used inside a **Violation**. Do not use the words interchangeably.
- **"fan-out"**: `miller-limit` used to say "fan-out" while counting importers. Importers of a module are its **fan-in** (**Miller Fan-in**); its own imports are its fan-out, which no built-in rule caps.
- **"stage"** used to mean the purity rank (`@stratum-stage`, `stratum/stage-purity`). It now means only the folder-structure evolution **Stage** of a **Container**; the rank is **Purity**.
- **"module"** in folder-structure is a folder-level unit: a **Container**. In Stratum a **Module** is one file.
- **"container"** used to mean the **Visualizer**'s per-layer box. It now means any unit of structure; a layer root is one kind of **Container**. A DI container (see **DI edge**) is unrelated.
- **"slice"**: a **Slice** is a **Container** of a `slices: true` **Layer**. The `slice` key of an **Import Policy** is a glob of directories inside which imports are allowed; it predates **Slices** and does not need them.
- **"scope"** covers a **Rule Scope**, a **Visibility Scope** and a **Report Scope**. Always qualify it.
- **"plugin"**: a **Plugin** is a **Rhai Script** that adds a **Custom Rule**; a **Harness Plugin** installs Stratum into Claude Code. Qualify when both are in play.
- **"rule"** covers both **Built-in Rules** and **Custom Rules**. They report the same **Violation** shape but differ in where they come from: built-ins are Rust, custom rules are **Rhai Scripts**.
- **"layer"** sometimes meant a workspace crate boundary. Reserve **Layer** for the methodology concept and say "crate" for Rust crates.
- **"methodology"** vs **"Stratum"**: **Stratum** is the tool; FSD is one **Methodology**, shipped as a **Preset**.
- **"violation"** vs **"error"**: **Violation** is the object; `error` is a value of **Severity**.
- **"finding"**: the code calls a **Violation** in its reported form (slug, root-relative paths) a `Finding`. In prose say **Violation**.
- **"fix"** vs **"suggestion"**: a **Violation** carries only a textual **Suggestion**. Do not speak of `fix.edits` or "auto-fix": that is a deliberate non-feature.
