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

## Methodology

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Methodology** | The configurable contract the tool enforces: **Layers**, **Visibility Scopes**, **Stages**, **Cross-Entity Patterns**. | architecture style, convention, paradigm |
| **Stratum** | The default **Methodology**. Ships as a config plus a plugin. | Stratum architecture, the methodology |
| **Methodology Bundle** | A distributable package that defines a **Methodology**: config plus plugins. | preset, profile, ruleset |
| **Layer** | A named group of **Modules** with its own visibility and dependency direction. | tier, level, slice |
| **Stage** | A purity rank from 1 to 4 that controls what a **Module** is allowed to depend on. | phase, tier, rank |
| **Visibility Scope** | The set of **Modules** allowed to import a given **Module**. `_shared` is one such scope. | access scope, boundary |
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
| **Violation** | One reported **Rule** infraction. Carries `file:line`, severity, a message, and an optional fix. | error, issue, finding |
| **Severity** | One of `error`, `warning`, `info`, `off`. | level, priority |
| **Suggested Fix** | A code action on a **Violation** that an IDE can apply automatically. | autofix, quick fix |
| **Self-Applicability** | The invariant that the tool reports zero **Violations** on its own codebase. | dogfooding, self-check |

## Tooling components

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Stratum Tooling** | The whole product: **Linter**, **LSP Server**, **Visualizer**, and the shared Rust core. | the toolchain, the suite |
| **Linter** | The CLI binary `stratum-lint`. Scans a project and reports **Violations**. | scanner, checker |
| **LSP Server** | `stratum-lsp`. The IDE-facing server that streams **Violations** as you type. | language server |
| **Visualizer** | `stratum-visualizer`. The Tauri app that renders the **Compound DAG** and its **Violations**. | viewer, graph UI |
| **Rule Engine** | The subsystem that registers and runs **Rules**. | rule runner |
| **Extractor** | A language adapter that parses source and produces **Modules** and **Imports**. | parser adapter, frontend |
| **Snapshot** | A JSON capture of the **Compound DAG** at a point in time. | dump, export, graph file |
| **Config** | `stratum.config.jsonc`. The declarative file that picks a **Methodology** and tunes **Rules** and **Severity**. | settings, configuration |
| **Override** | A glob-scoped block inside the **Config** that adjusts **Rule** behavior for matching paths. | exception, scoped rule |

## Extensibility

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Plugin** | A user extension that delivers **Custom Rules**. Two flavors: **WASM** and **Rhai**. | extension, addon |
| **WASM Plugin** | A compiled, sandboxed **Plugin** loaded through wasmtime. | wasm rule |
| **Rhai Script** | A **Plugin** written in the Rhai DSL. Hot-reloadable, no build step. | dsl rule, scripted rule |
| **Plugin API** | The types and methods exposed to **Plugins** for graph access and **Violation** reporting. | extension API, SDK |

## Visualization

| Term | Definition | Aliases to avoid |
|------|-----------|-----------------|
| **Toggle** | A UI control that hides or shows a **Layer**, a **Module**, or an **Edge** type. | filter, switch |
| **LOD** | Level-of-Detail rendering. Drops labels and details when you zoom out. | detail level |
| **Spatial Culling** | Skipping the render of **Modules** that fall outside the viewport. | viewport culling |
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

## Example dialogue

> **Dev:** "I'm adding a `runtime` **Edge** from a higher **Layer** down to a lower one. **Violation** or not?"
> **Architect:** "Depends on the **Methodology**. **Stratum** says **Stage** 4 can't depend on **Stage** 1, period. The **Edge** kind doesn't save you. So yeah, that's a **Violation** at `error`."
> **Dev:** "What about a one-off exception for legacy code?"
> **Architect:** "Don't reach for a **Custom Rule**. Put an **Override** in the **Config**, scoped by glob. **Custom Rules** are for invariants that aren't expressible yet, like 'no two **Modules** under `_shared` may share a name'. A **Rhai Script** handles most of those. **WASM Plugin** if it's heavy."
> **Dev:** "In the **Visualizer**, does the **Violation** show up on the **Edge** or the **Module**?"
> **Architect:** "Both. **Edge** goes red, both endpoint **Modules** get a red outline. Click either and the detail panel opens with the **Suggested Fix**."

## Flagged ambiguities

- **"graph"** was used loosely in the PRD for at least three things: the **Compound DAG**, the dependency DAG on its own, and a **Snapshot**. Pick **Compound DAG** for the in-memory thing and **Snapshot** for the file on disk.
- **"rule"** covered both **Built-in Rules** and **Custom Rules**. They speak the same **Plugin API**, but they differ in where they come from and how they're shipped. Qualify the term when that matters.
- **"plugin"** covered both **WASM Plugins** and **Rhai Scripts**. Both are **Plugins**, but they have different performance and packaging stories. Be specific when discussing builds or distribution.
- **"layer"** in the PRD sometimes meant a methodological **Layer** and sometimes a workspace crate boundary. Reserve **Layer** for the methodology concept; call crate boundaries **Crates** or **Workspace Modules**.
- **"methodology"** vs **"Stratum"**: **Stratum** is one concrete **Methodology**. The core does not know about **Stratum**; it only knows the **Methodology** contract.
- **"violation"** vs **"error"**: **Violation** is the object. `error` is a value of **Severity**. Don't conflate them.
