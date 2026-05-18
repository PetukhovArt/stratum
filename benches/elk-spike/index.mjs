import ELK from 'elkjs/lib/elk.bundled.js';
import { performance } from 'node:perf_hooks';
import { mkdirSync, writeFileSync } from 'node:fs';

const SIZES = [1000, 2000, 5000, 10000];
const LAYER_COUNT = 5;
const EDGES_PER_NODE = 3;

function buildGraph(nodeCount) {
  const layers = Array.from({ length: LAYER_COUNT }, (_, i) => ({
    id: `layer_${i}`,
    children: [],
    layoutOptions: { 'elk.algorithm': 'layered' },
  }));

  const allNodes = [];
  for (let i = 0; i < nodeCount; i += 1) {
    const layerIdx = i % LAYER_COUNT;
    const node = { id: `n_${i}`, width: 80, height: 30 };
    layers[layerIdx].children.push(node);
    allNodes.push({ ...node, layerIdx });
  }

  const edges = [];
  for (let i = 0; i < nodeCount; i += 1) {
    const fromLayer = allNodes[i].layerIdx;
    for (let e = 0; e < EDGES_PER_NODE; e += 1) {
      const targetLayer = (fromLayer + 1 + e) % LAYER_COUNT;
      if (targetLayer <= fromLayer) continue;
      const candidates = allNodes.filter((n) => n.layerIdx === targetLayer);
      if (candidates.length === 0) continue;
      const target = candidates[i % candidates.length];
      edges.push({ id: `e_${i}_${e}`, sources: [allNodes[i].id], targets: [target.id] });
    }
  }

  return {
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'DOWN',
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
    },
    children: layers,
    edges,
  };
}

async function runOnce(nodeCount) {
  const elk = new ELK();
  const graph = buildGraph(nodeCount);
  const t0 = performance.now();
  await elk.layout(graph);
  const t1 = performance.now();
  return t1 - t0;
}

async function main() {
  const results = [];
  for (const n of SIZES) {
    await runOnce(n);
    const samples = [];
    for (let i = 0; i < 3; i += 1) {
      const t = await runOnce(n);
      samples.push(t);
    }
    const avg = samples.reduce((a, b) => a + b, 0) / samples.length;
    const min = Math.min(...samples);
    const max = Math.max(...samples);
    results.push({ nodes: n, avg_ms: avg, min_ms: min, max_ms: max, samples });
    console.log(`nodes=${n.toString().padStart(6)}  avg=${avg.toFixed(1)}ms  min=${min.toFixed(1)}ms  max=${max.toFixed(1)}ms`);
  }

  mkdirSync('results', { recursive: true });
  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  writeFileSync(`results/elk-bench-${stamp}.json`, JSON.stringify({
    timestamp: new Date().toISOString(),
    layerCount: LAYER_COUNT,
    edgesPerNode: EDGES_PER_NODE,
    results,
  }, null, 2));

  const twoK = results.find((r) => r.nodes === 2000);
  if (!twoK) throw new Error('missing 2K result');
  if (twoK.avg_ms > 3000) {
    console.error(`FAIL: 2K layout averaged ${twoK.avg_ms.toFixed(0)}ms (budget 3000ms).`);
    console.error('Phase 8 scope must reduce: trigger sliced-view earlier or replace ELK.');
    process.exit(2);
  }
  console.log('PASS: 2K layout within 3s budget.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
