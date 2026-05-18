# Ubiquitous Language

## Graph structure

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Compound DAG** | The in-memory model: a nesting tree, a dependency DAG, and visibility scopes over the same set of nodes. | compound graph, architecture graph, dep graph |
| **Module** | One source unit (a file or an SFC) that appears as a node in the **Compound DAG**. Carries `id`, `path`, `container`, `layer`, `stage`, `visibility`. | file, component, unit |
| **Import** | A directed dependency from one **Module** to another. After resolution; pre-resolution form is a **Raw Import**. | dependency, link, reference |
| **Raw Import** (new) | A parser-emitted import edge before any `ModuleId` is assigned: literal `specifier`, byte `span`, `EdgeKind`, and a `type_only` flag. Lives in `stratum-parser-ts`; the `stratum-graph` crate (Phase 2) turns it into an **Edge**. | unresolved import, pre-edge |
| **Edge** | A typed connection between two **Modules**. Three kinds: `static`, `DI`, `runtime`. | arrow, line, connection |
| **Static edge** | An **Edge** created by a compile-time `import`/`export from`/dynamic `import()`. (Note: dynamic `import()` is currently classified `Static` by the OXC visitor — see `imports.rs`.) | static import |
| **DI edge** | An **Edge** wired through a composition root or container. | injection, wiring |
| **Runtime edge** | An **Edge** established at runtime: events, dynamic load. Not yet emitted by `stratum-parser-ts`. | event link, dynamic dep |
| **Type-only Import** (new) | A **Raw Import** marked `type_only` (e.g. `import type { X } from '...'`). Still recorded, so visibility rules can run on it; runtime-oriented rules filter it out. | erased import, types-only |
| **Cycle** | A strongly-connected component in the dependency DAG. Found via Tarjan SCC. | loop, recursion |
| **Container** | A compound node in the **Visualizer** that wraps the **Modules** of one **Layer**. | group, box, cluster |
| **Compound Graph Snapshot** | The opaque value returned by the `compound_graph` query on the **Architecture Database**. As of Phase 2 it carries the serialised **Graph Snapshot** as a JSON string (`{ json: String }`); the typed value lives in `stratum-graph`. The newtype avoids a `stratum-core → stratum-graph` dep cycle. | graph value, dag snapshot |
| **Graph Snapshot** (new) | The versioned, deterministic JSON shape `GraphSnapshot { version, modules, containers, layers, edges }` produced by `stratum_graph::snapshot_of(&CompoundGraph)`. Consumed by the **Visualizer** and by `compound_graph` callers. `version` bumps require a coordinated frontend release. | snapshot v1, on-wire graph |
| **Source Location** | A 1-based `line:column` pointer carried inside a **Violation**. Distinct from a **Source Span**. | position, line/col |
| **Source Span** | A byte-offset half-open range `[start, end)` into source text, used by the parser to point at AST fragments. Convertible to a **Source Location** via `offset_to_line_col`. | byte range, AST span |
| **Graph Builder** (new) | `stratum_graph::GraphBuilder` — walks a project root, picks files the **Language Extractor** handles, assigns each to its longest-matching **Layer**, then a second pass resolves **Raw Imports** through the **Path Resolver** and adds typed **Edges**. The Phase 2 entry point that materialises the **Compound DAG**. | graph constructor, dag builder |
| **Layer Assignment** (new) | The longest-matching-prefix mapping from a module's project-relative path to a `LayerId`. Files outside any configured **Layer** are dropped from the **Compound DAG** in Phase 2 (Phase 3 will surface them as a violation). | layer mapping, layer assign |
| **Miller Fan-out** (new) | A graph metric: the count of distinct dependents of a **Module** that live outside its own **Container**. Feeds the future `miller-limit` rule. Implemented as `stratum_graph::metrics::miller_fanout`. | cross-container fanout |
| **Depth Ratio** (new) | A graph metric: `container_internal_count / public_exports`. Rewards encapsulation. `public_exports` is supplied externally until Phase 5 wires real export counts in. Implemented as `stratum_graph::metrics::depth_ratio`. | encapsulation ratio |

## Methodology

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Methodology** | The configurable contract the tool enforces: **Layers**, **Visibility Scopes**, **Stages**, **Cross-Entity Patterns**. | architecture style, convention, paradigm |
| **Stratum** | The default **Methodology**. Ships as a config plus a plugin. | Stratum architecture, the methodology |
| **Methodology Bundle** | A distributable package that defines a **Methodology**: config plus plugins. | preset, profile, ruleset |
| **Layer** | A named group of **Modules** with its own visibility and a `depends_on: Vec<LayerId>` declaration. | tier, level, slice |
| **Stage** | A purity rank from 1 (most pure) to 4 (impure). A **Module** at stage `s` may depend on another at stage `t` iff `t <= s` — purer code cannot reach impurer code. | phase, tier, rank |
| **Visibility Scope** | The set of **Modules** allowed to import a given **Module**. Four variants: `Container { id }`, `Layer { id }`, `Public`, `Shared`. | access scope, boundary |
| **Cross-Entity Pattern** | A structural rule that spans more than one **Module** or **Layer**. | macro-pattern, structural rule |
| **Deep Module** | A **Module** whose public surface is small relative to what it does internally. | thick module, encapsulated module |
| **Erosion** | The buildup of **Violations** that wear down architectural integrity. | rot, drift, decay |

## Metrics

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Metric** | A number computed over the **Compound DAG**. | indicator, stat |
| **Depth Ratio** | A **Module**'s public surface divided by its internal complexity. | encapsulation ratio |
| **Miller Limit** | The cap on items a single **Module** or **Layer** should expose. About 4, to match working memory. | cognitive limit, fan-out cap |
| **Promotion Pressure** | A **Metric** that flags when a **Module** belongs in a higher **Layer**. | promotion score |

## Rules and diagnostics

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Rule** | A check that runs over the **Compound DAG** and emits zero or more **Violations**. | check, lint rule, validator |
| **Built-in Rule** | A **Rule** that ships inside the tool's core crates. | native rule, internal rule |
| **Custom Rule** | A user-written **Rule**. Delivered as a **WASM Plugin** or a **Rhai Script**. | user rule, extension |
| **Rule Scope** | The input shape a **Rule** operates over: a **Module**, a **Container**, or the whole **Project**. Drives per-scope Salsa memoization. | rule input, scope kind |
| **Violation** | One reported **Rule** infraction. Carries `rule`, `severity`, `message`, `file`, `location` (**Source Location**), `modules`, an optional `edge`, and an optional `suggestion`. | error, issue, finding |
| **Severity** | One of `error`, `warning`, `info`, `off`. Only `error` fails the build (`Severity::fails_build`). | level, priority |
| **Suggestion** | Human-readable advice attached to a **Violation**. **Never** an auto-applied fix in v1 (PRD decision #10) — the `Violation` struct deliberately has no `fix.edits` field. | autofix, quick fix, suggested fix |
| **Parser Diagnostic** (new) | A recovered parser-level error attached to **Extracted Data**. Distinct from a **Violation**: it's a syntax-level artifact, not a methodology infraction. | parse warning, syntax issue |
| **Self-Applicability** | The invariant that the tool reports zero **Violations** on its own codebase. | dogfooding, self-check |
| **Rule Registry** (new) | `RuleRegistry` plus the type-erased `DynRule` trait object (`DynRuleAdapter` wraps each concrete `Rule`). `RuleRegistry::with_builtins()` pre-registers the five baseline Stratum rules. | rule list, rule store |
| **Effective Rules** (new) | The rule set that applies to a single file after **Override** resolution. `EffectiveRules { rules: BTreeMap<slug, RuleConfig> }`, produced by `resolve_for_file(config, &path)`. | per-file rules, resolved rules |
| **Override Block** (new) | A JSONC `overrides[]` entry: `files: Vec<String>` glob list + per-rule overrides. Applied in array order, last-matching-wins per rule key. | overrides entry, scoped block |
| **Project Scope** (new) | The `ProjectScope` marker type used by whole-graph **Rules** (`no-circular-deps`). Singleton — `RuleScope::enumerate(graph)` returns `vec![ProjectScope]`. | global scope, project-wide |
| **Config Hash** (new) | `rule_config_hash(severity, options) -> u64` over the XXH3 algorithm. Phase 4 Salsa keys per-rule queries by `(rule_id, scope_value, config_hash)`. Stable across runs on the same input. | rule key hash, salsa key |
| **`run_all`** (new) | The Phase 3 orchestrator `stratum_rules::run_all(graph, config, project_root) -> Result<Vec<Violation>, ConfigError>`. Runs every rule once with base options, then re-stamps per-file severity from **Effective Rules**. Phase 3 limitation: per-file *options* overrides are not yet honoured — Phase 4 fixes via Salsa. | runner, orchestrator |

## Tooling components

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Stratum Tooling** | The whole product: **Linter**, **LSP Server**, **Visualizer**, and the shared Rust core. | the toolchain, the suite |
| **Linter** | The CLI binary `stratum-lint`. Scans a project and reports **Violations**. | scanner, checker |
| **LSP Server** | `stratum-lsp`. The IDE-facing server that streams **Violations** as you type. | language server |
| **Visualizer** | The browser-based UI for the **Compound DAG** and its **Violations**, served from `stratum-lint visualize` by an embedded HTTP server (axum + `rust-embed`). | viewer, graph UI, Tauri app |
| **Architecture Database** | The Salsa trait `ArchitectureDatabase`. The entire downstream-visible API of `stratum-core` — three deep queries (`compound_graph`, `violations`, `violations_for_file`). | db, query layer |
| **Stratum DB** | `StratumDb`, the concrete `salsa::Database` implementation of **Architecture Database**. Phase 0 returns placeholder values; Phase 2+ wires real queries. | salsa db |
| **`stratum-graph` Crate** (new) | The Phase 2 crate that builds the **Compound DAG** from **Extracted Data** and exposes graph algorithms (topo sort, Tarjan SCC, direct dependents/dependencies) and graph metrics. Depends on `stratum-core` and `stratum-parser-ts`. | graph crate |
| **`stratum-config` Crate** (new) | The Phase 3 crate that parses `stratum.config.jsonc` via `jsonc-parser`, generates a JSON Schema via `schemars`, and resolves per-file **Effective Rules** through `resolve_for_file`. Depends only on `stratum-core`. | config crate |
| **`stratum-rules` Crate** (new) | The Phase 3 crate that defines the `Rule`/`RuleScope` traits, the **Rule Registry**, the five baseline **Built-in Rules**, and the `run_all` orchestrator. Depends on `stratum-core`, `stratum-graph`, `stratum-config`. | rules crate |
| **Public Surface** | The deliberately narrow set of types and queries `stratum-core` exports. Intermediate Salsa queries stay `pub(crate)` (PRD decision D6). | API surface, exported API |
| **Project** | A Salsa input identifying a project root by `root: PathBuf` and `id: ProjectId`. Every query on the **Architecture Database** is parameterized on a **Project**. | workspace, repo, root |
| **Source File** (new) | A crate-private Salsa input (`SourceFile { path: Utf8PathBuf, text: String }`) holding one file's UTF-8 source. Only `stratum-graph` and `stratum-lint` inside the workspace read or set it. | file input, source input |
| **Extracted File** (new) | A crate-private Salsa input keyed by **Source File**, holding the parsed module data for that file. Set by the orchestration crate after translating from **Extracted Data** to avoid a `stratum-core → stratum-parser-ts` dep cycle. | parsed file, extracted input |
| **Rule Engine** | The subsystem that registers and runs **Rules**. | rule runner |
| **Language Extractor** (updated) | The trait `LanguageExtractor` — one impl per language. `handles(path) -> bool` plus `extract(path, source) -> Result<ExtractedData, ExtractError>`. Phase 1 ships `OxcTsExtractor` for `.ts/.tsx/.js/.jsx/.mjs/.cjs`. | parser adapter, frontend, extractor |
| **Extracted Data** (new) | The parser-side output for one source file: `source_path`, `imports: Vec<RawImport>`, and `diagnostics: Vec<ParserDiagnostic>`. The shape **Language Extractor** implementations produce. Don't confuse with **Extracted File** (the Salsa input). | parser result, extraction output |
| **OXC TS Extractor** (new) | `OxcTsExtractor` — the Phase 1 TypeScript/JavaScript **Language Extractor**, built on the OXC parser and AST visitor. | TS parser, ts extractor |
| **Path Resolver** (new) | `PathResolver` — a project-scoped wrapper over `oxc_resolver::Resolver`. Honours `tsconfig.json` `paths` aliases and resolves a specifier from one file's directory to an absolute UTF-8 path. | module resolver, tsconfig resolver |
| **Snapshot** | A JSON capture of the **Compound DAG** at a point in time — the on-disk artifact consumed by the **Visualizer**. Distinct from the in-memory **Compound Graph Snapshot**. | dump, export, graph file |
| **Config** | `stratum.config.jsonc`. The declarative file that picks a **Methodology** and tunes **Rules** and **Severity**. | settings, configuration |
| **Override** | A glob-scoped block inside the **Config** that adjusts **Rule** behavior for matching paths. Last-matching-wins by array order. | exception, scoped rule |

## Extensibility

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Plugin** | A user extension that delivers **Custom Rules**. Two flavors: **WASM** and **Rhai**. | extension, addon |
| **WASM Plugin** | A compiled, sandboxed **Plugin** loaded through wasmtime. | wasm rule |
| **Rhai Script** | A **Plugin** written in the Rhai DSL. Hot-reloadable, no build step. | dsl rule, scripted rule |
| **Plugin API** | The types and methods exposed to **Plugins** for graph access and **Violation** reporting. | extension API, SDK |
| **Module View** | The read-only projection of a **Module** that **Rhai Scripts** see: id, path, **Layer**, **Container**, **Stage**, imports, dependents, metadata. | rhai module, plugin module |

## Visualization

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Toggle** | A UI control that hides or shows a **Layer**, a **Module**, or an **Edge** type. | filter, switch |
| **LOD** | Level-of-Detail rendering. Drops labels and details when you zoom out. | detail level |
| **Spatial Culling** | Skipping the render of **Modules** that fall outside the viewport. | viewport culling |
| **Sliced View** | A reduced **Visualizer** mode showing only **Layers** and top-level **Containers**, with drill-down on click. Activates above the layout-library node-count threshold (provisional ≤500 nodes, pending ADR 0001). | overview mode, lite view |
| **Fixture Project** | A reference project under `tests/fixtures/` used as input for tests and benchmarks (e.g. `tiny-ts` for the **Path Resolver**). | sample project, test repo |

## Relationships

- A **Module** belongs to exactly one **Layer**, sits in exactly one **Container**, and has exactly one **Stage** and one **Visibility Scope**.
- An **Edge** is always typed as one **Edge Kind**: `static`, `DI`, or `runtime`.
- A **Raw Import** is the parser-side precursor to an **Edge**: it carries a textual `specifier` and a **Source Span**, but no `ModuleId`. The **Graph Builder** in **`stratum-graph` Crate** resolves it to an **Edge** via the **Path Resolver**.
- The **Graph Builder** materialises the **Compound DAG**. **Layer Assignment** drops files outside any configured **Layer**; the resulting graph is then serialised to a **Graph Snapshot** for the **Visualizer** and for the `compound_graph` query return value.
- A **Stage** at `s` may depend on a **Stage** at `t` iff `t ≤ s` — purer code cannot reach impurer code.
- A **Rule** reads the **Compound DAG** and emits zero or more **Violations**.
- A **Violation** has one **Severity**, one **Source Location**, and points at one or more **Modules** (and optionally one **Edge**).
- A **Methodology** is made of **Layers**, **Visibility Scopes**, **Stages**, **Cross-Entity Patterns**, and **Rules**.
- **Stratum** is one **Methodology** among many. The **Rule Engine** has no opinion about which one you pick.
- The **Linter**, the **LSP Server**, and the **Visualizer** share the same **Violation** type via the **Snapshot** contract.
- A **Plugin**, **WASM** or **Rhai**, registers **Custom Rules** through the **Plugin API**.
- Every query on the **Architecture Database** takes a **Project** as input. Per-file queries also take a path.
- A **Language Extractor** consumes raw source text and produces **Extracted Data**. The orchestration crate translates that into the crate-private **Extracted File** Salsa input keyed by a **Source File**.
- A **Rule** operates over exactly one **Rule Scope**, which determines its Salsa memoization key.

## Example dialogue

> **Dev:** "I'm adding `.vue` SFC support. Do I write a new **Language Extractor** or extend `OxcTsExtractor`?"
> **Architect:** "New impl. **Language Extractor** is a trait per language — `OxcTsExtractor` handles the TS/JS/JSX/TSX/MJS/CJS family. Write `VueExtractor`. Its `handles()` returns true for `.vue`, and `extract()` produces **Extracted Data** with the same `RawImport` shape."
> **Dev:** "Does `extract` resolve specifiers to file paths?"
> **Architect:** "No. **Language Extractor** only emits **Raw Imports** — literal `specifier` strings and **Source Spans**. The **Path Resolver** turns `'~shared/api/http'` into `/abs/path/src/shared/api/http.ts`. Resolution happens in `stratum-graph` (Phase 2), not in the parser."
> **Dev:** "What about `import type {...}`? Throw it away?"
> **Architect:** "No. It's a **Type-only Import** — recorded with `type_only: true`. Visibility rules still want to see them, since exposing a type across a forbidden boundary is a real violation. Rules that only care about runtime coupling filter on the flag."
> **Dev:** "And the parser-recovered syntax errors? Are those **Violations**?"
> **Architect:** "No — those are **Parser Diagnostics**, attached to the **Extracted Data**. They are syntax-level. A **Violation** is a methodology-level infraction emitted by a **Rule**. Don't conflate them in the LSP surface."
> **Dev:** "OK, and inside Salsa — `SourceFile` vs `ExtractedFile`?"
> **Architect:** "Both are crate-private Salsa inputs. **Source File** holds the raw UTF-8 text keyed by path. **Extracted File** holds the parsed module data, keyed by a **Source File**. The orchestration crate (Phase 4) translates **Extracted Data** from `stratum-parser-ts` into the **Extracted File** input — that hop exists to keep `stratum-core` from depending on `stratum-parser-ts`."

## Flagged ambiguities

- **"graph"** was used loosely in the PRD for at least three things: the **Compound DAG**, the dependency DAG on its own, and a **Snapshot**. Pick **Compound DAG** for the in-memory thing and **Snapshot** for the file on disk.
- **"snapshot"**: collision between the on-disk JSON **Snapshot** and the in-memory **Compound Graph Snapshot** returned by the `compound_graph` query. Always qualify: "JSON Snapshot" vs "in-memory Compound Graph Snapshot" when context is ambiguous.
- **"import"** (new): three things can be called an "import" — a **Raw Import** (parser-side, has a `specifier` string), an **Edge** (resolved, has `ModuleId`s), and the JS-keyword `import` statement in source. Use **Raw Import** for the parser side and **Edge** for the resolved side; reserve the bare word "import" for the source-language construct.
- **"extracted file"** vs **"extracted data"** (new): **Extracted Data** is the parser-side struct returned by a **Language Extractor**. **Extracted File** is the crate-private Salsa input. They carry related information but are different types and live in different crates — the orchestration crate translates one into the other.
- **"span"** vs **"location"** (new): a **Source Span** is a byte range `[start, end)` used inside the parser. A **Source Location** is a 1-based `line:column` pointer used inside a **Violation**. Convert with `offset_to_line_col`; do not use the words interchangeably.
- **"extractor"** (updated): the trait is **Language Extractor**; a specific impl is named like **OXC TS Extractor**. The word "extractor" alone is ambiguous — qualify when the difference matters.
- **"static edge"** (new): for now the parser classifies *every* `import`/`export from`/dynamic `import()` as `EdgeKind::Static`. Dynamic-`import()` reclassification (to `Runtime`) is a Phase-2-or-later decision; don't read "Static" today as "synchronous only".
- **"rule"** covered both **Built-in Rules** and **Custom Rules**. They speak the same **Plugin API**, but they differ in where they come from and how they're shipped. Qualify the term when that matters.
- **"plugin"** covered both **WASM Plugins** and **Rhai Scripts**. Both are **Plugins**, but they have different performance and packaging stories. Be specific when discussing builds or distribution.
- **"layer"** in the PRD sometimes meant a methodological **Layer** and sometimes a workspace crate boundary. Reserve **Layer** for the methodology concept; call crate boundaries **Crates** or **Workspace Modules**.
- **"methodology"** vs **"Stratum"**: **Stratum** is one concrete **Methodology**. The core does not know about **Stratum**; it only knows the **Methodology** contract.
- **"violation"** vs **"error"**: **Violation** is the object. `error` is a value of **Severity**. Don't conflate them. And neither is a **Parser Diagnostic** — that's a recovered syntax issue, not a methodology infraction.
- **"fix"** vs **"suggestion"**: there is **no fix object** in v1 — a **Violation** carries only a textual **Suggestion**. Don't speak of `fix.edits` or "auto-fix" — that's a deliberate non-feature (PRD decision #10).
- **"project"**: **Project** in this glossary is the Salsa input identifying a workspace root. Don't confuse with "the project" as in "this whole tool" — use **Stratum Tooling** for the product.
