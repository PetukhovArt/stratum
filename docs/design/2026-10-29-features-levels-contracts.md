# Features, levels and contracts in the visualizer

Status: proposal, 2026-10-29. Slices S1–S6 below, each shippable alone.

## 0. Goal

1. Build from the code a map of how the current structure implements features.
2. Filter that map to one or more features for visual analysis.
3. Move between levels: system → container → component → code. Entering a container shows how its components connect. Going up to the system level shows the APIs between systems as arrows. Each arrow is a contract or a call, and opening it shows the code with the relevant lines highlighted.
4. Publish the result as a static site (GitLab Pages, GitHub Pages) and as standard C4 text (Mermaid, Structurizr, PlantUML).

Not a goal: guessing features from folder names, runtime tracing, a general C4 editor.

## 1. Current code

- The parser already keeps what a contract needs, but the snapshot drops it. An `Import` has `location`, `bindings` (the imported names, `type_only`) and `target` (`crates/stratum-core/src/types.rs:42-51`). `ExportDecl` has `name`, `location`, and `origin` with the re-export chain (`types.rs:96-112`). `SnapshotEdge` is only `{ from, to, kind }` (`crates/stratum-graph/src/snapshot.rs:59-64`).
- Calls are recorded only for patterns that a rule asks for (`TrackedCalls`, `crates/stratum-parser-ts/src/imports.rs:32,160`). A `Call` has a name and a location but no arguments and no enclosing declaration (`types.rs:35-39`).
- Reachability over imports already exists in `reachability-policy`.
- The server has `/api/snapshot`, `/api/violations`, `/api/context` and `/api/drift`, all computed once at start (`crates/stratum-lint/src/commands/visualize.rs:131-158`). Views are registered in `frontend/.../src/views/index.ts`.
- Stratum parses only TS and Vue. There is no model of systems outside ROOT.

## 2. Usage first

```jsonc
{
  // Optional. Without it, every slice of the topmost sliced layer is one feature.
  "features": {
    "checkout": { "name": "Checkout", "entry": ["src/pages/checkout/**"] },
    "search":   { "entry": ["src/features/search/index.ts"] }
  },
  // Optional. Without it, the Architecture view starts at the container level.
  "systems": [
    { "id": "ui", "name": "Web UI", "self": true },
    { "id": "host", "name": "Desktop host", "root": "../desktop/src",
      "contracts": { "kind": "rust-attr", "attr": "tauri::command" },
      "calls": [{ "callee": "invoke", "name_arg": 0 }] },
    { "id": "api", "name": "Backend", "root": "../backend/proto",
      "contracts": { "kind": "proto" },
      "calls": [{ "factory": "createClient", "service_arg": 0 }] }
  ]
}
```

The CLI:

```sh
stratum-lint visualize [ROOT] [--feature ID]...      # opens with the filter set
stratum-lint features [ROOT] [--format json|text]    # features, their modules, shared modules
stratum-lint export [ROOT] --out public/ [--with-source] [--base REF]   # static site
stratum-lint c4 [ROOT] --format mermaid|structurizr|plantuml [--level system|container|component] [--feature ID]...
```

## 3. Where it lives in the visualizer

The work is not a separate page. It has three parts:

- **A new view `6` Architecture**, with zoom levels: System → Containers → Components → Code. Breadcrumbs at the top. `Enter` or a double click goes down into the selection. `Backspace` or a breadcrumb goes up. Levels:
  - **System**: one box per system, arrows for channels. The arrow label is the number of contracts in use. The details panel lists the contracts (rpc or command), each with its signature and its callers as `file:line`.
  - **Containers**: layers and slices of `self`, laid out by the Lanes code that Container map already uses. Other systems are drawn as ports at the edge, with arrows from the slices that call them.
  - **Components**: the modules of one container, grouped by segment, with import arrows.
  - **Code**: the source panel with the import line, the declaration and the call sites highlighted.
- **A feature filter in the header**, applied to every view. It is a multi-select chip. The views show only the union of the chosen features. A module that two or more chosen features use is marked "shared". "Hide common" also hides modules that more than k features use.
- **A Contract section in the details panel** of every edge, in every view. It lists the names that cross the edge, each resolved through barrels to the real declaration, with `file:line` that opens Code.

## 4. Model

- **Feature** = its entry modules plus everything reachable from them by static imports, type-only imports included. A module that belongs to one feature is **exclusive**. A module that belongs to more than one is **shared**.
- **Contract of an import edge** = the imported names. Each name is resolved through `ExportOrigin::Reexport` to its declaring module and line. A container bundle's contract is the union over its edges. That union is the public API of the slice that is actually used.
- **System**, **Channel** and **Contract** (cross-system):
  - The contract adapters are name-agnostic and configured:
    - `proto`: a line parser for `service S { rpc M(Req) returns (Res); }`.
    - `rust-attr`: functions under the given attribute, with their signature.
    - Later: `openapi` and `http-route`.
  - The call matchers on the TS side:
    - `callee` + `name_arg`: a string literal argument names the contract.
    - `factory` + `service_arg`: `const c = factory(Service, …)`, and then `c.method(…)` names `Service.method`.
  - The parser grows `Call.args` (literal strings and identifiers) and `Call.owner` (the enclosing top-level declaration). The owner lets a feature that imports `getUser` from an api module be credited with only the RPCs inside `getUser`, not with every RPC in that file.

## 5. Transport

- Snapshot stays v3 in S1–S3. Edge details are lazy:
  - `/api/edge?from=&to=` returns `{ imports: [{ line, names: [{ name, type_only, decl: { path, line } }] }] }`.
  - `/api/source?path=REL&line=N&context=K` returns text lines. Paths outside ROOT are allowed only under a declared `systems[].root`. They are addressed as `system-id:rel/path`.
- `/api/features` returns `{ features: [{ id, name, entry, modules }], shared: [moduleIdx] }`.
- `/api/systems` returns `{ systems, channels: [{ from, to, contracts: [{ id, signature, decl, callers: [{ module, line, owner }] }] }] }`, or 404 without `systems`. A `root` that is missing makes its system show "contracts unavailable" instead of failing the run.
- Static mode for S6: the frontend reads `./data/*.json` when `window.__STRATUM_STATIC__` is set. `/api/edge` becomes one `edges.json` holding all edges. `/api/source` is present only with `--with-source`. Without it, Code shows `file:line` and the signature, with no source text. `/api/context` is precomputed per module.

## 6. Code highlight

The highlighter is `highlight.js` core with TS, Rust and Protobuf, loaded as a lazy chunk when Code first opens. That costs about 60 KB against a budget of 1.5 MB (currently 385 KB). Shiki was rejected because of its size.

## 7. Slices

| Slice | What | Done when |
|---|---|---|
| S1 Edge contracts | `/api/edge` with barrel resolution, `/api/source`, the Contract section in Module focus, Container map bundles and Module set, the Code panel | Clicking a bundle between two slices lists the names that cross it, and each name opens its real declaration through an `index.ts` |
| S2 Features | `features` config and its default, `/api/features`, `stratum-lint features`, the header filter in all views | Choosing 2 features leaves only their union; shared modules are marked; the CLI prints the same sets |
| S3 Architecture view | View `6`, levels Containers → Components → Code, breadcrumbs, keys | From the top level, the declaration of a cross-slice name is reachable in ≤ 3 actions |
| S4 Systems | `systems` config, the `proto` and `rust-attr` adapters, call matchers, `Call.args/owner`, `/api/systems`, the System level, ports at Containers | With one feature in the filter, the System level shows only the RPCs and commands that feature reaches, each with caller `file:line` |
| S5 C4 text | `stratum-lint c4` for Mermaid C4 (it renders inline in GitLab and GitHub markdown), Structurizr DSL and C4-PlantUML; system and container levels, feature filter | The Mermaid output renders in a GitLab MR description; Structurizr Lite opens the DSL |
| S6 Static site | `stratum-lint export --out DIR`, static mode in the frontend, CI examples in `docs/ci.md` for GitLab Pages (`pages` job, `public/`) and GitHub Pages (`actions/deploy-pages`) | The exported folder opened with a plain file server works the same as `visualize`, apart from the Brief of files that do not exist yet |

Order: S1 → S2 → S3 → S4. S5 and S6 can start after S2. S5 needs S4 for the system level.

## 8. Acceptance

- Unit and e2e tests on `tests/fixtures/tiny-ts`, extended with a barrel, two features with a shared module, a fake `proto` root and a `rust-attr` root.
- **Bench** (`9`) gets canonical questions for each slice: answered in ≤ 3 actions, perf budgets unchanged.
- Live check: on a closed real project (TS/React, a Rust desktop host, a gRPC backend), with a local config outside its repo, run read-only.

## 9. Risks

- **Size of static export with source.** Source is opt-in (`--with-source`) because Pages may be public. Without it, nothing beyond paths, names and signatures leaves the repo.
- **Factory call matching.** Matching is per file only: the client must be created and used in the same module. A client passed between modules is not followed. This is reported in the System level as "N calls not attributed".
- **Feature closure through shared kernels.** Everything ends up reaching `shared`. "Hide common" and the exclusive/shared split handle that instead of cutting the traversal.
