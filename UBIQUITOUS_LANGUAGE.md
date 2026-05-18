# Ubiquitous Language

## Graph structure

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Compound DAG** | The in-memory model: a nesting tree, a dependency DAG, and visibility scopes over the same set of nodes. | compound graph, architecture graph, dep graph |
| **Module** | One source unit (a file or an SFC) that appears as a node in the **Compound DAG**. | file, component, unit |
| **Import** | A directed dependency from one **Module** to another. | dependency, link, reference |
| **Edge** | A typed connection between two **Modules**. Three kinds: `static`, `DI`, `runtime`. | arrow, line, connection |
| **Static edge** | An **Edge** created by a compile-time `import`/`export`. | static import |
| **DI edge** | An **Edge** wired through a composition root or container. | injection, wiring |
| **Runtime edge** | An **Edge** established at runtime: events, dynamic load. | event link, dynamic dep |
| **Cycle** | A strongly-connected component in the dependency DAG. Found via Tarjan SCC. | loop, recursion |
| **Container** | A compound node in the **Visualizer** that wraps the **Modules** of one **Layer**. | group, box, cluster |
| **Compound Graph Snapshot** (new) | The opaque in-memory value returned by the `compound_graph` query on the **Architecture Database**. Phase 0 ships a placeholder; the real shape lands in `stratum-graph`. | graph value, dag snapshot |
| **Source Location** (new) | A 1-based `file:line:column` pointer carried inside a **Violation**. | position, span anchor |

## Methodology

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Methodology** | The configurable contract the tool enforces: **Layers**, **Visibility Scopes**, **Stages**, **Cross-Entity Patterns**. | architecture style, convention, paradigm |
| **Stratum** | The default **Methodology**. Ships as a config plus a plugin. | Stratum architecture, the methodology |
| **Methodology Bundle** | A distributable package that defines a **Methodology**: config plus plugins. | preset, profile, ruleset |
| **Layer** | A named group of **Modules** with its own visibility and dependency direction. | tier, level, slice |
| **Stage** | A purity rank from 1 to 4 that controls what a **Module** is allowed to depend on. | phase, tier, rank |
| **Visibility Scope** | The set of **Modules** allowed to import a given **Module**. Has four variants: **Container**, **Layer**, **Public**, **Shared**. | access scope, boundary |
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
| **Rule Scope** (new) | The input shape a **Rule** operates over: a **Module**, a **Container**, or the whole **Project**. Drives per-scope Salsa memoization. | rule input, scope kind |
| **Violation** | One reported **Rule** infraction. Carries a **Source Location**, a **Severity**, a message, and an optional **Suggestion**. | error, issue, finding |
| **Severity** | One of `error`, `warning`, `info`, `off`. | level, priority |
| **Suggestion** (updated) | Human-readable advice attached to a **Violation**. **Never** an auto-applied fix in v1 (PRD decision #10). | autofix, quick fix, suggested fix |
| **Self-Applicability** | The invariant that the tool reports zero **Violations** on its own codebase. | dogfooding, self-check |

## Tooling components

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Stratum Tooling** | The whole product: **Linter**, **LSP Server**, **Visualizer**, and the shared Rust core. | the toolchain, the suite |
| **Linter** | The CLI binary `stratum-lint`. Scans a project and reports **Violations**. | scanner, checker |
| **LSP Server** | `stratum-lsp`. The IDE-facing server that streams **Violations** as you type. | language server |
| **Visualizer** (updated) | The browser-based UI for the **Compound DAG** and its **Violations**, served from `stratum-lint visualize` by an embedded HTTP server (axum + `rust-embed`). | viewer, graph UI, Tauri app |
| **Architecture Database** (new) | The Salsa trait `ArchitectureDatabase`. The entire downstream-visible API of `stratum-core` — three deep queries (`compound_graph`, `violations`, `violations_for_file`). | db, query layer |
| **Public Surface** (new) | The deliberately narrow set of types and queries `stratum-core` exports. Intermediate Salsa queries stay `pub(crate)` (PRD decision D6). | API surface, exported API |
| **Project** (new) | A Salsa input that identifies a project root by path and `ProjectId`. Every query on the **Architecture Database** is parameterized on a **Project**. | workspace, repo, root |
| **Rule Engine** | The subsystem that registers and runs **Rules**. | rule runner |
| **Extractor** | A language adapter that parses source and produces **Modules** and **Imports**. | parser adapter, frontend |
| **Snapshot** (updated) | A JSON capture of the **Compound DAG** at a point in time — the on-disk artifact consumed by the **Visualizer**. Distinct from the in-memory **Compound Graph Snapshot**. | dump, export, graph file |
| **Config** | `stratum.config.jsonc`. The declarative file that picks a **Methodology** and tunes **Rules** and **Severity**. | settings, configuration |
| **Override** | A glob-scoped block inside the **Config** that adjusts **Rule** behavior for matching paths. Last-matching-wins by array order. | exception, scoped rule |

## Extensibility

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Plugin** | A user extension that delivers **Custom Rules**. Two flavors: **WASM** and **Rhai**. | extension, addon |
| **WASM Plugin** | A compiled, sandboxed **Plugin** loaded through wasmtime. | wasm rule |
| **Rhai Script** | A **Plugin** written in the Rhai DSL. Hot-reloadable, no build step. | dsl rule, scripted rule |
| **Plugin API** | The types and methods exposed to **Plugins** for graph access and **Violation** reporting. | extension API, SDK |
| **Module View** (new) | The read-only projection of a **Module** that **Rhai Scripts** see: id, path, **Layer**, **Container**, **Stage**, imports, dependents, metadata. | rhai module, plugin module |

## Visualization

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Toggle** | A UI control that hides or shows a **Layer**, a **Module**, or an **Edge** type. | filter, switch |
| **LOD** | Level-of-Detail rendering. Drops labels and details when you zoom out. | detail level |
| **Spatial Culling** | Skipping the render of **Modules** that fall outside the viewport. | viewport culling |
| **Sliced View** (new) | A reduced **Visualizer** mode showing only **Layers** and top-level **Containers**, with drill-down on click. Activates above the layout-library node-count threshold (provisional ≤500 nodes, pending ADR 0001). | overview mode, lite view |
| **Fixture Project** | A reference project under `tests/fixtures/` used as input for tests and benchmarks. | sample project, test repo |

## Relationships

- A **Module** belongs to exactly one **Layer** and has exactly one **Stage**.
- An **Import** is always typed as one **Edge** kind: `static`, `DI`, or `runtime`.
- A **Rule** reads the **Compound DAG** and emits zero or more **Violations**.
- A **Violation** has one **Severity** and points at one or more **Modules** (and optionally an **Edge**).
- A **Methodology** is made of **Layers**, **Visibility Scopes**, **Stages**, **Cross-Entity Patterns**, and **Rules**.
- **Stratum** is one **Methodology** among many. The **Rule Engine** has no opinion about which one you pick.
- The **Linter**, the **LSP Server**, and the **Visualizer** share the same **Violation** type via the **Snapshot** contract.
- A **Plugin**, **WASM** or **Rhai**, registers **Custom Rules** through the **Plugin API**.
- Every query on the **Architecture Database** takes a **Project** as input. Per-file queries also take a path. (new)
- A **Rule** operates over exactly one **Rule Scope**, which determines its Salsa memoization key. (new)

## Example dialogue

> **Dev:** "I added a **Rule** that crosses **Modules** in different **Layers**. What **Rule Scope** should it use?"
> **Architect:** "Whole **Project**. **Rule Scope** is **Module** if you're checking one **Module**'s imports, **Container** if it's a per-folder thing, **Project** for anything cross-cutting. The **Architecture Database** memoizes by `(rule_id, scope_value)` either way."
> **Dev:** "Does the **Violation** get auto-fixed in the **LSP Server**?"
> **Architect:** "No. A **Violation** carries an optional **Suggestion**, which is plain text. **Never** an `edits`-style auto-fix in v1 — decision #10. The hover panel shows the **Suggestion**, the dev applies it by hand."
> **Dev:** "And in the **Visualizer**? Does it load the whole **Compound DAG**?"
> **Architect:** "On disk you get a **Snapshot** — JSON. The browser hydrates that into a working model. If the **Project** is over the layout threshold, the **Visualizer** falls back to **Sliced View**: only **Layers** and top **Containers**, drill-down on click. ELK.js can't render 2K+ compound nodes inside three seconds — see ADR 0001."
> **Dev:** "So `CompoundGraphSnapshot` in `stratum-core` is the same thing as a **Snapshot**?"
> **Architect:** "No, and that's the trap. The **Compound Graph Snapshot** is the in-memory Salsa value from the `compound_graph` query. The **Snapshot** is the JSON file on disk. They serialize into each other but they're not the same type."

## Flagged ambiguities

- **"graph"** was used loosely in the PRD for at least three things: the **Compound DAG**, the dependency DAG on its own, and a **Snapshot**. Pick **Compound DAG** for the in-memory thing and **Snapshot** for the file on disk.
- **"snapshot"** (new): collision between the on-disk JSON **Snapshot** and the in-memory **Compound Graph Snapshot** returned by the `compound_graph` query. Always qualify: "JSON Snapshot" vs "in-memory Compound Graph Snapshot" when context is ambiguous.
- **"rule"** covered both **Built-in Rules** and **Custom Rules**. They speak the same **Plugin API**, but they differ in where they come from and how they're shipped. Qualify the term when that matters.
- **"plugin"** covered both **WASM Plugins** and **Rhai Scripts**. Both are **Plugins**, but they have different performance and packaging stories. Be specific when discussing builds or distribution.
- **"layer"** in the PRD sometimes meant a methodological **Layer** and sometimes a workspace crate boundary. Reserve **Layer** for the methodology concept; call crate boundaries **Crates** or **Workspace Modules**.
- **"methodology"** vs **"Stratum"**: **Stratum** is one concrete **Methodology**. The core does not know about **Stratum**; it only knows the **Methodology** contract.
- **"violation"** vs **"error"**: **Violation** is the object. `error` is a value of **Severity**. Don't conflate them.
- **"fix"** vs **"suggestion"** (new): there is **no fix object** in v1 — a **Violation** carries only a textual **Suggestion**. Don't speak of `fix.edits` or "auto-fix" — that's a deliberate non-feature (PRD decision #10).
- **"project"** (new): **Project** in this glossary is the Salsa input identifying a workspace root. Don't confuse with "the project" as in "this whole tool" — use **Stratum Tooling** for the product.
