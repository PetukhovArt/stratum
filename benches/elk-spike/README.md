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
| _to be filled by Task 13_ | 1000 |  |  |  |  |
| _to be filled by Task 13_ | 2000 |  |  |  |  |
| _to be filled by Task 13_ | 5000 |  |  |  |  |
| _to be filled by Task 13_ | 10000 |  |  |  |  |
