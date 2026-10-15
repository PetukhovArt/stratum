# Generic structure: containers, stage, purity

Status: implemented, 2026-10-15, in slices S0–S5 (8dded36 to c7cb60a). Written as a proposal against base `main` f003638 (`project.ignore` merged); the line references below point at that base and have since moved. Sources: folder-structure `fsd-spec.md` section "Linting" (~836-949) and `folder-structure-architecture/*.html`.

Deviations from this proposal:
- `stage-mixing` reads `Container.segment_dirs`, recorded by placement, instead of segment names injected into its options by the runner; the rule takes no options (3d92cdd).
- The runner caps severity instead of overwriting it: rules run at `error` and each finding is clamped to the configured severity, so a rule may downgrade its own findings. `container-breadth` reports private-zone folders at `info` at most.
- An entry file directly in a sliced layer's root is the layer's entry, never a file slice (b21fbe6).
- `slice-fan-in` is scoped per slice (`Scope = ContainerId` over `Slice` containers), not per module.
- Snapshot v2 containers carry `kind` and `entry` as well as `path` and `stage`.
- The `context` brief also names the enclosing slice and adds a `may not import` line built from the active `sibling-isolation` and `public-api` rules.
- Zero-config applies `stratum:recommended` with `public-api`, `sibling-isolation` and `container-breadth` off, because without declared layers and structure they flood the first run (c7cb60a).

## 0. Current code

- There is one container per layer. Its id is the layer id and its parent is `None` (`crates/stratum-graph/src/builder.rs:128-140`), and every module gets that container (`builder.rs:167`). Slices do not exist in the model.
- Folder names are hardcoded: `/entities/` and `/features/` (`crates/stratum-rules/src/builtin/cross_entity_pattern.rs:122,131`), `/index.ts` (`:88`) and `/_shared/` (`visibility_scope.rs:87`). Every module is `Public` (`crates/stratum-engine/src/pipeline.rs:67`), so `visibility-scope` never fires.
- `Stage` is a purity rank (`crates/stratum-core/src/stage.rs:4-35`) that comes from `// @stratum-stage N` (`crates/stratum-parser-ts/src/annotations.rs:6`) and defaults to 2 (`pipeline.rs:66`).
- The visualizer reads the snapshot field `module.stage`, which holds purity, but displays it with folder-structure labels (`frontend/.../render/adapt.ts:150`, `components/Header.tsx:126-172`). It also computes the evolution stage itself from the folder tree (`adapt.ts:223-275`). These are two different meanings under one field.
- Container-based metrics compare `Module.container` (`crates/stratum-graph/src/metrics/miller.rs:15`, `promotion.rs:18`, `depth_ratio.rs:24`). `depth-ratio` keeps its own copy of the container size (`crates/stratum-rules/src/builtin/depth_ratio.rs:72`).

## 1. Usage first

The user's config. FSD names appear only in the preset or the user's file, never in code:
```jsonc
{ "version": 1, "project": { "extends": "stratum:recommended" },
  "structure": { "segments": ["model", "ui", "api", "lib"], "private": ["_local"] },
  "layers": [
    { "id": "screens", "path": "src/screens", "depends_on": ["domain", "kit"], "slices": true },
    { "id": "domain",  "path": "src/domain",  "depends_on": ["kit"],           "slices": true },
    { "id": "kit",     "path": "src/kit" } ],
  "rules": { "stratum/restricted-calls": { "severity": "error", "options": [
    { "calls": ["inject", "provide"], "allow": ["**/_local/*-scope.ts"],
      "message": "Use useXxxScope()/provideXxxScope() from _local/<name>-scope.ts" } ] } } }
```
Call sites that fix the graph interface:
```rust
// public_api.rs: deep import into the outermost container that holds the target but not the importer
let outer = graph.containers_of(to).filter(|c| c.kind != ContainerKind::Layer)
    .take_while(|c| !graph.containers_of(from).any(|f| f.id == c.id)).last();
if let Some(c) = outer && c.entry != Some(to) { out.push(deep_import(c, from, to)); }
// metrics/promotion.rs: "another container" now means another slice
.filter(|d| graph.outer_container(*d) != graph.outer_container(m))
// commands/context.rs:106: a file that does not exist yet still gets layer, container and stage
let container = graph.container_at(&file_abs);              // replaces assign_layer
// pipeline.rs:63: config to build input, one type with no copy
BuildConfig { structure: config.structure.clone(), tracked_calls: stratum_rules::tracked_calls(config), .. }
```

## 2. Unit model

**Placement** (`crates/stratum-graph/src/placement.rs`) absorbs `layer_assignment.rs:11` and turns the sorted file list into nodes. The derivation rules:
1. Each layer gets one `Layer` container at the layer path. This is today's default.
2. With `slices: true`, each direct child of the layer path becomes a `Slice` container, whether it is a folder or a source file. A folder slice is a slice with or without an entry file. A file slice has stage 1 and is its own entry.
3. Below a `Slice`, `SubUnit` or `Nested` container, a direct child folder that holds an entry file becomes a `SubUnit`. A deeper entry-bearing folder becomes `Nested`: one that sits under a segment or a collection folder, or one that is a direct child of an unsliced layer root. A `SubUnit` is isolated from its siblings. A `Nested` container is not.
4. A segment folder or a private-zone folder is never a container, even when it has an entry file. A folder without an entry file is a collection folder. Its files belong to the enclosing container.
5. `Module.container` is the innermost container that encloses the module. The `Container.parent` chain always ends at a `Layer` container.
6. Stage per container:
   - stage 1 for a file slice;
   - stage 4 when it has a direct `SubUnit` child;
   - stage 3 when it has a direct segment folder;
   - stage 2 otherwise.
   Segments and sub-units together give stage 4, and `stage-mixing` flags it. The private zone counts toward neither.
7. A module under a private-zone folder `D/<name>/...` gets `VisibilityScope::Private { root: D }`. The innermost zone wins. Names match by whole path component, never by substring.

Interface:
- `Container` (`crates/stratum-core/src/types.rs:23`) gains `path: PathBuf`, `kind: ContainerKind {Layer, Slice, SubUnit, Nested}`, `stage: Stage` and `entry: Option<ModuleId>`. `path` has the same form as `Module.path`.
- `Layer` gains `slices: bool`.
- `VisibilityScope::Shared` (`visibility.rs:16-17`) is replaced by `Private { root: PathBuf }`.
- `ContainerId` stays an opaque `u32`. Layer containers take the layer index and the rest follow in path order. Ids are deterministic for a given path set and are not stable across path-set changes.
- `CompoundGraph` (`graph.rs`) gains three readers:
  - `containers_of(m)` goes from the innermost container to the `Layer` container;
  - `outer_container(m)` returns the slice, else the layer container;
  - `container_at(path)` returns the innermost enclosing container, and the path does not have to exist.
- `assign_layer` leaves the public exports (`crates/stratum-graph/src/lib.rs:16`). Its only outside caller is `context.rs:106`.
- `modules_in_container` (`graph.rs:48`) has no production caller, so it is deleted.
- Invariant: placement is a function of the path set. `update_module` (incremental) never moves a module between containers. Adding, removing or renaming a file requires `build()`. For example, adding an `index.ts` turns a collection folder into a container.

What metrics measure now:
- Miller fan-in (in flight: `miller_fan_in`) and promotion pressure count importers whose `outer_container` differs. Unsliced layers behave as they do today. In sliced layers both measure "used by other slices", so imports between sub-units of one slice no longer count.
- `depth_ratio` uses the module count of the container subtree. The rule and the metric share one helper and drop the copy at `depth_ratio.rs:72`.

## 3. Config

```jsonc
"structure": {                                     // new top-level key, every field optional
  "entries":  ["index.ts", "index.tsx", "index.js", "index.jsx", "index.vue"], // exact file names
  "segments": [],                                  // folder names; the stratum:fsd preset sets model/ui/api/lib/config
  "private":  ["_shared"]                          // private-zone folder names
},
"layers": [{ "id": "...", "path": "...", "depends_on": [], "slices": false }]  // slices: new, default false
```

- `Structure` lives in `stratum-core` (`structure.rs`, serde and JsonSchema). `Config.structure` (`crates/stratum-config/src/config.rs:9-23`) and `BuildConfig` both hold that same type, so no copy has to be kept in sync.
- `extends` merge:
  - `"structure"` joins `merge_fields` next to `"project"` (`crates/stratum-config/src/extends.rs:96`). The merge is per field and a child array replaces the parent's array, like `depends_on` does. `ignore` appends, but these lists are names, not filters, so replacing is the right fit.
  - `slices` merges with the rest of a layer by `id`.
- Schema: regenerate with `STRATUM_UPDATE_SCHEMA=1` (`crates/stratum-config/tests/schema_snapshot.rs:8-21`). Rule options stay untyped in the schema.

check-yagni, one line per option:
- `entries`: the user asked for it, and `public-api.ts` naming exists. Kept.
- `segments`: has two real values, `[]` and the FSD list. Kept.
- `private`: the spec says `shared/` (spec:566) and the HTML says `_shared/` (iso:119). Kept.
- `slices`: four FSD layers are true and the rest are false. Kept as a bool. The glob form lost because no value other than `*` is in sight.
- `sibling-isolation.allow`: dropped. An `overrides` block that sets severity `off` per importer glob covers entity cross-imports.
- `container-breadth`'s second limit for homogeneous folders (folder-structure ~20): dropped, because a collection folder is not a container and is never checked.
- A configurable default purity: dropped. It stays 2 (`pipeline.rs:66`).

## 4. Rules

| Slug (id) | Scope | Semantics | recommended | fsd |
|---|---|---|---|---|
| `purity` (3, was `stage-purity`) | Module | Purity rank `t` may be imported from rank `s` only when `t <= s`. | warning | inherit |
| `visibility-scope` (6) | Module | Imports into `Private{root}` must come from inside `root`. This is the private zone. | error | error |
| `public-api` (11) | Module | An import from outside container C (not `Layer`) must target `C.entry`. If C has no entry, the message says so. | warning | error |
| `sibling-isolation` (12) | Module | An edge between two different `Slice`/`SubUnit` children of one container is a violation, at any depth. | warning | error |
| `restricted-calls` (13) | Module | A call to a listed name outside the `allow` globs is a violation, reported at the call's line. | off | off |
| `stage-mixing` (14) | Container | Segment folders next to `SubUnit` folders. | off | error |
| `container-breadth` (15) | Container | More than `max_kinds` (6) kinds directly in a non-`Layer` container. The message names the next stage. | info | warning |
| `slice-fan-in` (7, was `cross-entity-pattern`) | Container | More than `max_fan_in` (5, was `max_fanout`) distinct importing slices into one `Slice`. | off | warning |

Rule details:
- **Kind.** Child files collapse by the stem before the first dot, so `foo.ts`, `foo.test.ts` and `foo.vue` count as one kind. Each child folder counts as one kind. The entry file and private zones do not count.
- **`restricted-calls` options.** A list of `{calls: [string], allow: [glob], message?: string}` with forward-slash globs relative to the root, following the `import-policy` precedent (`import_policy.rs:41-54`).
- **Severity.** Slugs keep the `stratum/` prefix. `stage-mixing` stays off in recommended because with no segments it never fires. One edge can trip both `public-api` and `sibling-isolation`; the two stay independent.
- **`explain`.** Each new slug gets a `RuleDoc` (`crates/stratum-rules/src/docs.rs:11-67`) with what, why, fix and options. The `why` text cites folder-structure: outside only via `index.ts` (bas:228); a private zone is visible to direct siblings only (iso:148).
- **`context`.** The `Brief` (`crates/stratum-lint/src/commands/context.rs:27-43`) changes:
  - `stage` becomes `purity` and `max_import_stage` becomes `max_import_purity`;
  - `container` becomes `{path, kind, stage, entry}`;
  - a new `private_zones` field lists the zones this file can see.
  The agent line reads: `container src/screens/home (slice, stage 4, entry index.ts): import other containers only via their entry`.

## 5. Parser

- **Call names.**
  - `ImportVisitor::visit_call_expression` already visits every call in its single AST pass (`crates/stratum-parser-ts/src/imports.rs:113-126`).
  - It records an `Identifier` callee whose name is in the tracked set. It also records a local alias of an import whose imported name is tracked: in `import {inject as i}`, `i(...)` matches.
  - The output is `ExtractedData.calls: Vec<RawCall{name, span}>` (`crates/stratum-parser-ts/src/extractor.rs:25-34`).
  - Extractors take the set at construction (`OxcTsExtractor::new(calls)`, `VueExtractor::new(calls)`), so `LanguageExtractor` (`extractor.rs:51-62`) does not change.
  - With an empty set the visitor checks one `is_empty` per call and allocates nothing. With N tracked names it does one hash lookup per call plus allocations for matches only. Verify on `benches/cold_lint`.
  - The builder stores the result as `Module.calls: Vec<Call{name, location}>`. It converts the span to a line while it still has the source.
  - `update_module` refreshes `calls` along with the edges.
- **The set's owner.** `stratum_rules::tracked_calls(&Config)` returns the union of `calls` over the `restricted-calls` options in `rules` and `overrides`. The pipeline never parses rule options itself.
- **Entry files and folder kinds.** No parser change. Placement derives them from paths.
- **Exports.** Owned by the in-flight branch. `RawCall` sits next to its `exports` field.

## 6. Rename checklist (stage to purity)

- [x] `crates/stratum-core/src/stage.rs` becomes `purity.rs`: `Stage` -> `Purity`, `InvalidStage` -> `InvalidPurity`. Also update `lib.rs:17,27`, `tests/properties.rs:4-26`, and `Module.stage` -> `Module.purity` (`types.rs:18`).
- [x] `// @stratum-stage N` becomes `// @stratum-purity N`:
  - `extract_stage` -> `extract_purity` (`annotations.rs:6,14`, `crates/stratum-parser-ts/src/lib.rs:12`, `oxc_ts.rs:4,52,57`);
  - `ExtractedData.stage` -> `purity` (`extractor.rs:31-33`);
  - the Vue extractor (`crates/stratum-parser-vue/src/script_extractor.rs:5,54`);
  - the fixtures `tests/fixtures/tiny-ts-violations/src/shared/api/http.ts:1` and `.../features/cart/cart.tsx:1`.
- [x] `BuildConfig.default_stage` -> `default_purity` (`builder.rs:54,169,286,378`, `pipeline.rs:66`, the three benches, every `Stage::new(2)` in tests). Also the incremental `update_module`.
- [x] `stratum/stage-purity` -> `stratum/purity`:
  - `stage_purity.rs` -> `purity.rs`;
  - `STAGE_PURITY` -> `PURITY` (`crates/stratum-rules/src/ids.rs:5,19,36`);
  - `registry.rs:163,169`, `docs.rs:25-28`, `presets.rs:16`, `reporters/agent.rs:206`;
  - the three fixture configs and the snapshots: rerun `cargo insta`.
- [x] `context.rs`: `DEFAULT_STAGE` (`:23`) and the brief fields (`:33-36,136-144,230-240`), with snapshots.
- [x] Rhai `ModuleView.stage` -> `purity` (`crates/stratum-plugins-rhai/src/module_view.rs:18,68,82,106`) and `docs/custom-rules.md:113`.
- [x] Snapshot v2 (`crates/stratum-graph/src/snapshot.rs:11,30,66`):
  - `module.stage` -> `module.purity`;
  - `container` gains `path` and `stage`.
- [x] Frontend (`frontend/stratum-visualizer-frontend/src/`):
  - in `types.ts:14-27,66`, the snapshot types change and the version becomes 2;
  - in `adapt.ts`, `:150` becomes `purity`, `:122` takes `c.stage`, and `computeStage`/`containerStage` (`:223-275`) are deleted;
  - `state.ts:27` `colorMode` becomes `'layer'|'stage'|'purity'`, with stage for containers and purity for modules;
  - `Header.tsx:126-172` becomes a purity filter with purity labels;
  - `Panels.tsx:73-86` keeps its folder-structure labels for containers only. (Panels.tsx was later split; the details panel shows the stage as a facet chip.)
- [x] Docs: `docs/visualizer.md:47,71,85`, the `UBIQUITOUS_LANGUAGE.md` rows (section 8), and a new CHANGELOG entry. The historical plans under `docs/superpowers/` were deleted afterwards; git keeps them.
- [x] Baseline fingerprints keyed by the old slug go stale. That is accepted, because there are no users yet.

## 7. Work split (after the export and incremental branches merge)

1. **S0 rename** (serial): every checklist item except snapshot v2 and the frontend. The tree compiles with no `Stage` type at all, so a half-done rename cannot silently mean evolution.
2. **S1 seam** (serial, one agent), which owns:
   - `stratum-core` `types.rs`, `stage.rs` (new evolution `Stage`), `structure.rs`, `visibility.rs`;
   - `stratum-config` `config.rs`, `extends.rs`, `presets.rs`, schema;
   - `stratum-graph` `placement.rs`, `builder.rs`, `graph.rs`, `lib.rs`;
   - `pipeline.rs`, and `context.rs:106` only.
   It also registers all new rules: `ids.rs`, `registry.rs`, `options.rs`, the final `docs.rs` text, and `check` bodies that return `vec![]`. It adds `tracked_calls` returning an empty set and `git mv cross_entity_pattern.rs slice_fan_in.rs` with the slug change only.
3. Then in parallel, each slice owning only its files:
   - **S2 imports**: `public_api.rs`, `sibling_isolation.rs`, `visibility_scope.rs`.
   - **S3 structure and metrics**: `stage_mixing.rs`, `container_breadth.rs`, the `slice_fan_in.rs` body, `metrics/{miller,promotion,depth_ratio}.rs`, `builtin/depth_ratio.rs`.
   - **S4 calls**: parser-ts `imports.rs`/`oxc_ts.rs`/`extractor.rs`, parser-vue `script_extractor.rs`, `builder.rs` `scan()` plus `builder/incremental.rs`, `restricted_calls.rs`, the `tracked_calls` body.
   - **S5 surfaces**: `snapshot.rs` v2 plus the frontend (one coordinated bump), the `context.rs` brief, docs, the glossary, CHANGELOG.
4. **S5 tail** (after S2-S4): turn the new rules on in `tests/fixtures/exemplar-stratum-app/stratum.config.jsonc` (with `slices` and `structure`) and re-baseline `crates/stratum-rules/tests/exemplar.rs` once. Until then each slice proves its rule with unit tests on a hand-built `CompoundGraph`.

## 8. Glossary edits (`UBIQUITOUS_LANGUAGE.md`, in the file's current table format)

- **Purity** (replaces the Stage row at :43): rank 1-4, where rank `s` may import rank `t` only when `t <= s`. Avoid: stage, rank. Set by `@stratum-purity`.
- **Stage** (redefined): the folder-structure evolution stage of a container: 1 file, 2 flat folder, 3 segmented, 4 composed of sub-units. Computed from the tree and never annotated. Avoid: phase, level.
- **Container** (redefined at :17): a unit of structure. It is a layer root, a slice, a sub-unit or a nested container, and it has a path, a stage and an optional entry. Avoid: unit, box, group. folder-structure's "module" is a Container, not a Module (flagged ambiguity).
- **Slice**: a container at the first level of a layer declared `slices: true`. It may be a file. Avoid: feature, package.
- **Sub-unit**: an entry-bearing folder directly under a container. It is isolated from its siblings and makes its parent stage 4. Avoid: sub-feature, submodule.
- **Segment**: a folder named in `structure.segments`, which groups a container's files by purpose. It is never a container. Avoid: layer folder.
- **Entry**: the file named in `structure.entries` that is a container's only import target from outside. Avoid: barrel, public API file.
- **Private Zone**: a folder named in `structure.private`, visible only inside its parent folder's subtree. Avoid: local shared.
- **Collection Folder**: a folder with no entry. Its files belong to the enclosing container. Avoid: group folder.
- Edit Module (:8), the `@stratum-stage` annotation (:27), Methodology (:39), Visibility Scope (:44: `Private` replaces `Shared`), Cross-Entity Pattern (:45, becomes Slice Fan-in), Miller Fan-out (:24) and Module View (:122), plus the relationships at :140-147.
- Note: the house glossary format is blocks with Russian definitions. This file uses English tables. The conversion is a separate decision.

## 9. Deletion test and red flags

- **placement.rs.** Without it, entry, segment, stage and visibility logic would repeat in the builder, `context.rs` and four rules. It earns its keep.
  - Split ownership: the frontend `computeStage` is a second owner of stage, so it is deleted (S5).
  - Hand-synced list: names exist only in `Structure`. Rules read `Container`, never config names.
- **The three `CompoundGraph` readers.** Without them every rule re-walks `parent` chains. They are kept. `modules_in_container` fails the test, so it is deleted.
- **`Structure` in core.** One type serves config and build. The alternative was a `stratum-config` copy mapped in `pipeline.rs`, which would be a hand-synced list.
- **`tracked_calls`.** Without it, `pipeline.rs` would parse `restricted-calls` options, which leaks a rule's option format. It is kept.
- **`slice-fan-in` against `miller-limit`.** They differ only in granularity: slices into a slice versus modules into a module. It barely survives, and it is a merge candidate if counting by `outer_container` makes them converge.
- **`restricted-calls` against extending `import-policy`.** A `calls` key would mix first-match semantics across two inputs (edges and calls). That alternative is rejected below.
- **Two ways to do one task.** `VisibilityScope::Container/Layer` are never assigned (`pipeline.rs:67`). They are dead-code candidates and out of scope here.

## 10. Summary

**Settled:**
- `Container` fields, `ContainerKind`, and the derivation rules in section 2;
- the three graph readers;
- `Structure` and `Layer.slices`;
- the `VisibilityScope::Private` shape;
- the eight slugs and ids;
- `RawCall` and `tracked_calls`;
- snapshot v2 fields;
- S0 -> S1 -> {S2..S5}.

**Open:**
- Miller for private-zone folders (folder-structure spec:927, "dumping ground"). They are not containers, so `container-breadth` skips them.
- The stage-4 composer-cleanliness check (spec:925).
- Callees of the form `x.inject(...)`.
- Whether `index.vue` belongs in the default `entries`.

**Tradeoffs:**
- We accept that structure changes force a full rebuild, in exchange for placement being a pure function of the path set.
- We accept two violations on one edge (`public-api` plus `sibling-isolation`), in exchange for rules that can be switched independently.
- We accept that aliased namespace calls go unseen, in exchange for zero parser cost when `restricted-calls` is off.
- We accept that an unsliced layer root does not isolate its entry-bearing children, in exchange for no noise in kit-style layers.

**Alternatives that lost:**
- Declaring every unit by glob (`containers: "*/*"` per layer). It lost on depth: users would hand-maintain what entry files already say, and nested sub-units would need one glob per depth, which is the ESLint-boundaries trap at spec:847.
- Folding call checks into `import-policy`. It lost on locality.
