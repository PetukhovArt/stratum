# ELK Layout Spike (Phase 0)

Synthetic-graph benchmark for ELK.js compound-graph layout latency.
Validates PRD risk R5 ("ELK.js layout latency on >2K compound nodes")
before Phase 8 starts.

## Run

```bash
cd benches/elk-spike
npm install
npm run bench
```

Output goes to `results/elk-bench-<timestamp>.json`.

## Gate

The script exits non-zero if average 2K-node layout exceeds 3 seconds
(PRD US-9 budget). If that happens, Phase 8 must change scope:

- Lower the sliced-view threshold from 2K to whatever ELK can handle smoothly.
- OR replace ELK with dagre-wasm / custom Sugiyama / graphviz-wasm.
- OR accept a longer initial layout (e.g. show a spinner) and document it.

## Recorded results

| Date | Node count | Avg ms | Min ms | Max ms | Verdict |
|------|------------|--------|--------|--------|---------|
| 2026-05-18 | 1000 | 3836 | 3689 | 4022 | WARN |
| 2026-05-18 | 2000 | 13203 | 12960 | 13478 | FAIL |
| 2026-05-18 | 5000 | — | — | — | (not run — aborted after FAIL at 2K) |
| 2026-05-18 | 10000 | — | — | — | (not run — aborted after FAIL at 2K) |

## Phase 8 scope decision

ELK.js does not meet the 2K budget. 2K-node layout averaged ~13 s (budget: 3 s, 4× over).
Even 1K nodes averaged ~3.8 s, implying a real ceiling of ~500–700 nodes in the browser.

Phase 8 scope amended: sliced-view threshold lowered to ≤ 500 visible nodes.
Alternatives to evaluate before Phase 8 starts (tracked in ADR 0001):
- **dagre-wasm** — port of dagre to WASM, handles hierarchical layouts
- **graphviz-wasm** — svg output, needs a re-render step for interaction
- **Custom Sugiyama in Rust/WASM** — most control, highest implementation cost
