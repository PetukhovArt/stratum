# ADR 0001: ELK.js layout alternatives

**Status:** open
**Date:** 2026-05-18

## Context

Phase 0 ELK.js spike (see `benches/elk-spike/README.md`) showed:

- 1 K nodes → avg **3836 ms** (warn; already over the 3 s budget)
- 2 K nodes → avg **13 203 ms** (fail; 4.4× over the 3 s budget)

The machine: Windows 10, Node v22.12.0, `elkjs@0.9.x`, single ELK worker thread.
ELK's Java reference implementation is faster; the JS bundled port is the bottleneck.

The PRD US-9 success metric ("Initial layout time for 2K compound nodes < 3 s") is **not met** with ELK.js in the browser.

## Decision

Deferred. Candidates to evaluate before Phase 8 starts:

| Candidate | Pros | Cons |
|-----------|------|------|
| **dagre-wasm** | Same API family as dagre-d3, compound support exists | Less maintained, compound mode limited |
| **graphviz-wasm** | Mature algorithm, deterministic | SVG-only output, no interactive drag; needs re-render pass for PixiJS |
| **Custom Sugiyama (Rust→WASM)** | Full control, integrates with `stratum-graph` | Highest effort (2–3 weeks extra) |
| **ELK with Web Worker + partial layout** | Keeps ELK; off-thread so UI not blocked | Still slow; needs incremental layout API |
| **Lower sliced-view threshold to 300–500 nodes** | No library change; ship faster | Reduces visualizer utility on larger codebases |

## Consequences

- Phase 8 **cannot start** until this ADR resolves and a replacement is chosen.
- Until resolved, sliced-view threshold is provisionally lowered to ≤ 500 visible nodes.
- Roadmap needs a 1-week buffer before Phase 8 for layout library evaluation.
- Update this ADR with the chosen solution and close it before Phase 8 Task 1.
