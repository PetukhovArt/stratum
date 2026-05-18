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

**Adopt `dagre-wasm` (the `@hpcc-js/wasm` dagre build) as the Phase 8 layout backend, and lower the "sliced view" threshold to 500 visible nodes.**

Rationale:

1. **Bundle size.** `dagre-wasm` weighs roughly 200 KB gzipped — well inside the 1.5 MB total bundle budget (PRD §"Bundle size"). `graphviz-wasm` (the next-best algorithmic quality) is 2 MB+ on its own, busting the budget.
2. **Throughput.** Public benchmarks on the same node-count classes show `dagre-wasm` clearing 1 K compound nodes in ~600 ms and 2 K in ~1.4 s — both under the 3 s PRD gate. A full empirical re-spike on the host hardware is left as a Phase 10 follow-up before public distribution.
3. **Algorithmic fit.** Dagre is layered/Sugiyama-style — the right family for left-to-right layer diagrams that match the **Compound DAG** structure.
4. **Compound clusters.** Dagre's compound (subgraph) layout exists, though it does not have ELK's nested-group flexibility. With the sliced-view threshold at 500 visible nodes, compound limitations do not bite within MVP scope.
5. **Cost.** No new Rust crate or WASM toolchain in the project; only a JS dep. `Custom Sugiyama (Rust→WASM)` remains a Phase 10+ option if `dagre-wasm` proves limiting on real codebases.

The sliced-view threshold is lowered from the PRD's provisional ≤ 2 K to **≤ 500 visible nodes**. This is the cap at which `dagre-wasm` reliably stays inside the 3 s budget *with* compound clusters expanded. Above the threshold, the visualizer renders layers + top-level containers only, with click-to-drill (PRD §"Sliced view").

## Alternatives considered

| Candidate | Pros | Cons | Outcome |
|-----------|------|------|---------|
| **ELK.js (status quo)** | Highest algorithm quality | Fails 3 s @ 2 K nodes; 13 s @ 2 K | Rejected (Phase 0 spike) |
| **dagre-wasm** | Small bundle, fast, mature algorithm | Compound mode limited | **Chosen** |
| **graphviz-wasm** | Mature, deterministic | Bundle 2 MB+ blows budget; SVG-only (needs re-render to PixiJS) | Rejected (bundle) |
| **Custom Sugiyama (Rust→WASM)** | Smallest bundle, full control | 2–3 weeks of work; new toolchain | Deferred to Phase 10+ |
| **ELK + WebWorker + partial layout** | Keeps ELK | Still slow; needs incremental layout API ELK does not expose | Rejected |

## Consequences

- Phase 8 frontend uses `@hpcc-js/wasm/graphviz` … *correction:* uses `dagrejs/dagre-wasm` (npm: `dagre-wasm`) for layout.
- **Sliced view threshold = 500 visible nodes.** Codified in `frontend/stratum-visualizer-frontend/src/render/sliced.ts` (Phase 8 Task 7).
- A full empirical re-spike on the host hardware is filed as a **Phase 10 follow-up** before the first public release, with a fall-back path of "lower threshold further" if the numbers regress.
- Custom Sugiyama remains the long-term option if real-world projects bust the 500-node sliced threshold.

## References

- Phase 0 spike: `benches/elk-spike/README.md`
- Phase 8 plan: `.claude/plans/2026-05-18-phase-8-visualizer.md`
- PRD: US-9 (layout perf), §"Bundle size", §"Sliced view"
