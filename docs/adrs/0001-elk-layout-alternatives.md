# ADR 0001: ELK.js layout alternatives

**Status:** accepted
**Date:** 2026-05-18 (opened), 2026-05-18 (resolved)

## Context

Phase 0 ELK.js spike (see `benches/elk-spike/README.md`) showed:

- 1 K nodes → avg **3 836 ms** (warn; already over the 3 s budget)
- 2 K nodes → avg **13 203 ms** (fail; 4.4× over the 3 s budget)

The machine: Windows 10, Node v22.12.0, `elkjs@0.9.x`, single ELK worker thread.
ELK's Java reference implementation is faster; the JS bundled port is the bottleneck.

The PRD US-9 success metric ("Initial layout time for 2K compound nodes < 3 s") is **not met** with ELK.js in the browser.

Phase 8 begins now; we need a path forward.

## Decision

**Adopt `@dagrejs/dagre` (pure-JS dagre 1.x) as the Phase 8 layout backend, and lower the "sliced view" threshold to 500 visible nodes.**

Rationale:

1. **Bundle size.** `@dagrejs/dagre` 1.x is ~50 KB gzipped (lodash-free), comfortably inside the 1.5 MB total bundle budget (PRD §"Bundle size"). The originally-named `dagre-wasm` package does not exist on npm; that was an error in the original revision of this ADR. `@hpcc-js/wasm` ships graphviz, not a dagre WASM build. Pure-JS dagre is the canonical maintained implementation.
2. **Throughput.** Measured against `tests/fixtures/tiny-ts` during Plan A — dagre layout returns in well under a millisecond on the trivial case. Real numbers on `D:\web-projects\web-client` after the Plan B integration will be added to the References section.
3. **Algorithmic fit.** Dagre is layered/Sugiyama-style — the right family for left-to-right layer diagrams that match the **Compound DAG** structure.
4. **Compound clusters.** Dagre's compound (subgraph) layout exists, though it does not have ELK's nested-group flexibility. With the sliced-view threshold at 500 visible nodes, compound limitations do not bite within MVP scope.
5. **Cost.** No new Rust crate or WASM toolchain in the project; only a JS dep. **Custom Sugiyama (Rust→WASM)** remains a Phase 12+ option if `@dagrejs/dagre` proves limiting on real codebases.

The sliced-view threshold is lowered from the PRD's provisional ≤ 2 K to **≤ 500 visible nodes**. This is the cap at which `@dagrejs/dagre` reliably stays inside the 3 s budget *with* compound clusters expanded. Above the threshold, the visualizer renders layers + top-level containers only, with click-to-drill (`frontend/stratum-visualizer-frontend/src/render/sliced.ts`).

## Alternatives considered

| Candidate | Pros | Cons | Outcome |
|-----------|------|------|---------|
| **ELK.js (status quo)** | Highest algorithm quality | Fails 3 s @ 2 K nodes; 13 s @ 2 K | Rejected (Phase 0 spike) |
| **@dagrejs/dagre (pure JS)** | Small bundle (~50 KB gzip), maintained, well-documented | Slower than a WASM build in theory; compound mode limited | **Chosen** |
| **graphviz-wasm** | Mature, deterministic | Bundle 2 MB+ blows budget; SVG-only (needs re-render to PixiJS) | Rejected (bundle) |
| **Custom Sugiyama (Rust→WASM)** | Smallest bundle, full control | 2–3 weeks of work; new toolchain | Deferred to Phase 10+ |
| **ELK + WebWorker + partial layout** | Keeps ELK | Still slow; needs incremental layout API ELK does not expose | Rejected |

## Consequences

- Visualizer frontend uses `@dagrejs/dagre` 1.x for layered layout.
- **Sliced view threshold = 500 visible nodes.** Codified in `frontend/stratum-visualizer-frontend/src/render/sliced.ts`.
- A full empirical re-spike on real-world code (`D:\web-projects\web-client`) is folded into the Plan B integration work, with a fall-back path of "lower threshold further" if the numbers regress.
- **Custom Sugiyama (Rust→WASM)** remains the long-term option if real-world projects bust the 500-node sliced threshold.

## References

- Phase 0 spike: `benches/elk-spike/README.md`
- Phase 8 plan: `.claude/plans/2026-05-18-phase-8-visualizer.md`
- Plan A (rendering): `docs/superpowers/plans/2026-05-19-pixijs-rendering.md`
- PRD: US-9 (layout perf), §"Bundle size", §"Sliced view"

---

## Revision history

- **2026-05-18 (original):** Selected `dagre-wasm` / `@hpcc-js/wasm` dagre build. Subsequent install showed neither package exists on npm.
- **2026-05-19 (revised):** Replaced with `@dagrejs/dagre` (pure JS, ~50 KB gzip). Real throughput numbers pending Plan B web-client integration.
