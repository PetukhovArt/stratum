# PixiJS Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Phase 8 MVP placeholder in the visualizer frontend with a real, interactive dependency-graph renderer using PixiJS for drawing and `@dagrejs/dagre` for layered layout.

**Architecture:** SolidJS owns app state and routing. A `render/` module owns canvas. `layout.ts` is a pure function: `(GraphSnapshot, options) → PositionedGraph`. `scene.ts` initializes a PixiJS `Application` + `pixi-viewport` for pan/zoom. `nodes.ts` and `edges.ts` build sprites/Graphics from the positioned graph. `sliced.ts` decides whether to render the full graph or collapse to layers when visible node count exceeds 500 (per ADR-0001). The flow is one-way: snapshot → layout → scene mutate.

**Tech Stack:** TypeScript, SolidJS, PixiJS 8.x, `@dagrejs/dagre` (pure JS, ~50 KB gzip — replaces nonexistent `dagre-wasm`), `pixi-viewport` for pan/zoom, Vitest for unit tests, Playwright for E2E.

**Preliminary commit (not a task — do before Task 1):**
Update GitHub Actions to v5 across `.github/workflows/*.yml` so the Node 20 deprecation warning goes away. Specifically: `actions/checkout@v4` → `@v5`, `actions/setup-node@v4` → `@v5`, `actions/upload-artifact@v4` → `@v5`, `actions/download-artifact@v4` → `@v5`. Set `node-version: '22'` while you're at it (current LTS). Commit as `chore(ci): bump actions to v5 and Node 22 LTS`.

---

### Task 1: Add dependencies — `@dagrejs/dagre` and `pixi-viewport`

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/package.json`

- [ ] **Step 1: Update dependencies block**

```json
{
  "name": "stratum-visualizer-frontend",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:e2e": "playwright test"
  },
  "dependencies": {
    "solid-js": "^1.8.0",
    "pixi.js": "^8.0.0",
    "pixi-viewport": "^6.0.2",
    "@dagrejs/dagre": "^1.1.4"
  },
  "devDependencies": {
    "@playwright/test": "^1.45.0",
    "@types/dagrejs__dagre": "^1.1.0",
    "typescript": "^5.4.0",
    "vite": "^5.2.0",
    "vite-plugin-solid": "^2.10.0",
    "vitest": "^1.6.0"
  }
}
```

Note: `pixi-viewport@^6` is the line that supports PixiJS 8. Earlier majors are for PixiJS 7. If `@types/dagrejs__dagre` 404s, use `@dagrejs/dagre`'s built-in types (the package ships `.d.ts` since 1.x); remove the `@types/...` devDep.

- [ ] **Step 2: Install and verify lockfile**

Run:
```bash
cd frontend/stratum-visualizer-frontend
npm install --no-audit --no-fund
```

Expected: `package-lock.json` is created/updated, no errors. `node_modules/@dagrejs/dagre/package.json` exists.

- [ ] **Step 3: Verify bundle still fits budget after install**

Run:
```bash
cd frontend/stratum-visualizer-frontend
npm run build
du -sb dist
```

Expected: bundle stays under 1,572,864 bytes (1.5 MB). If close to ceiling, note current size in commit message.

- [ ] **Step 4: Commit**

```bash
git add frontend/stratum-visualizer-frontend/package.json frontend/stratum-visualizer-frontend/package-lock.json
git commit -m "feat(visualizer): add @dagrejs/dagre and pixi-viewport"
```

---

### Task 2: Layout module — pure function wrapping dagre

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/render/layout.ts`
- Create: `frontend/stratum-visualizer-frontend/src/render/layout.test.ts`

The layout module is the trickiest pure-data part of the renderer. Keep it framework-free so it's testable with plain Vitest. Read `src/types.ts` first to confirm `GraphSnapshot` field names — the test uses them.

- [ ] **Step 1: Write the failing test**

`frontend/stratum-visualizer-frontend/src/render/layout.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { computeLayout } from './layout';
import type { GraphSnapshot } from '../types';

const trivialSnapshot: GraphSnapshot = {
  nodes: [
    { id: 'a', label: 'A', layer: 0, kind: 'module', parent: null },
    { id: 'b', label: 'B', layer: 1, kind: 'module', parent: null },
  ],
  edges: [{ from: 'a', to: 'b', kind: 'import' }],
  layers: ['domain', 'app'],
};

describe('computeLayout', () => {
  it('returns one positioned node per input node', () => {
    const out = computeLayout(trivialSnapshot, { rankdir: 'LR' });
    expect(out.nodes).toHaveLength(2);
    expect(out.nodes.map(n => n.id).sort()).toEqual(['a', 'b']);
  });

  it('preserves edge endpoints', () => {
    const out = computeLayout(trivialSnapshot, { rankdir: 'LR' });
    expect(out.edges).toHaveLength(1);
    expect(out.edges[0].from).toBe('a');
    expect(out.edges[0].to).toBe('b');
    expect(out.edges[0].points.length).toBeGreaterThanOrEqual(2);
  });

  it('places target to the right of source under LR layout', () => {
    const out = computeLayout(trivialSnapshot, { rankdir: 'LR' });
    const a = out.nodes.find(n => n.id === 'a')!;
    const b = out.nodes.find(n => n.id === 'b')!;
    expect(b.x).toBeGreaterThan(a.x);
  });

  it('returns total bounds wide enough to fit all nodes', () => {
    const out = computeLayout(trivialSnapshot, { rankdir: 'LR' });
    const maxX = Math.max(...out.nodes.map(n => n.x + n.width / 2));
    expect(out.bounds.width).toBeGreaterThanOrEqual(maxX);
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/layout.test.ts
```

Expected: FAIL — `Cannot find module './layout'`.

- [ ] **Step 3: Implement `layout.ts`**

`frontend/stratum-visualizer-frontend/src/render/layout.ts`:
```ts
import dagre from '@dagrejs/dagre';
import type { GraphSnapshot } from '../types';

export interface LayoutOptions {
  rankdir: 'LR' | 'TB';
  nodeSep?: number;
  rankSep?: number;
  nodeWidth?: number;
  nodeHeight?: number;
}

export interface PositionedNode {
  id: string;
  label: string;
  layer: number;
  kind: string;
  parent: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface PositionedEdge {
  from: string;
  to: string;
  kind: string;
  points: Array<{ x: number; y: number }>;
}

export interface PositionedGraph {
  nodes: PositionedNode[];
  edges: PositionedEdge[];
  bounds: { width: number; height: number };
}

export function computeLayout(
  snapshot: GraphSnapshot,
  options: LayoutOptions,
): PositionedGraph {
  const g = new dagre.graphlib.Graph({ compound: true });
  g.setGraph({
    rankdir: options.rankdir,
    nodesep: options.nodeSep ?? 40,
    ranksep: options.rankSep ?? 80,
    marginx: 20,
    marginy: 20,
  });
  g.setDefaultEdgeLabel(() => ({}));

  const w = options.nodeWidth ?? 160;
  const h = options.nodeHeight ?? 40;

  for (const node of snapshot.nodes) {
    g.setNode(node.id, { width: w, height: h, label: node.label });
    if (node.parent) g.setParent(node.id, node.parent);
  }
  for (const edge of snapshot.edges) {
    g.setEdge(edge.from, edge.to);
  }

  dagre.layout(g);

  const nodes: PositionedNode[] = snapshot.nodes.map(n => {
    const laid = g.node(n.id);
    return {
      id: n.id,
      label: n.label,
      layer: n.layer,
      kind: n.kind,
      parent: n.parent,
      x: laid.x,
      y: laid.y,
      width: laid.width,
      height: laid.height,
    };
  });

  const edges: PositionedEdge[] = snapshot.edges.map(e => {
    const laid = g.edge(e.from, e.to);
    return {
      from: e.from,
      to: e.to,
      kind: e.kind,
      points: laid?.points ?? [],
    };
  });

  const graphLabel = g.graph();
  return {
    nodes,
    edges,
    bounds: {
      width: graphLabel.width ?? 0,
      height: graphLabel.height ?? 0,
    },
  };
}
```

- [ ] **Step 4: Run test, verify it passes**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/layout.test.ts
```

Expected: PASS, 4 assertions.

- [ ] **Step 5: Commit**

```bash
git add frontend/stratum-visualizer-frontend/src/render/layout.ts frontend/stratum-visualizer-frontend/src/render/layout.test.ts
git commit -m "feat(visualizer): layout module wrapping @dagrejs/dagre"
```

---

### Task 3: PixiJS scene + viewport bootstrap

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/render/scene.ts`
- Create: `frontend/stratum-visualizer-frontend/src/render/scene.test.ts`

Scene initializes the PixiJS `Application` against a host `HTMLCanvasElement`, wires `pixi-viewport` for pan/zoom, exposes a `mount` / `destroy` lifecycle. Tests use happy-dom / jsdom canvas stubs.

- [ ] **Step 1: Configure Vitest for canvas-friendly DOM**

Edit `frontend/stratum-visualizer-frontend/vite.config.ts` and add:
```ts
import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid()],
  test: {
    environment: 'happy-dom',
    setupFiles: ['./src/test/setup.ts'],
  },
});
```

Create `frontend/stratum-visualizer-frontend/src/test/setup.ts`:
```ts
import { vi } from 'vitest';

class FakeCanvasCtx {
  fillRect = vi.fn();
  clearRect = vi.fn();
  beginPath = vi.fn();
  moveTo = vi.fn();
  lineTo = vi.fn();
  stroke = vi.fn();
}

HTMLCanvasElement.prototype.getContext = vi.fn(() => new FakeCanvasCtx()) as unknown as typeof HTMLCanvasElement.prototype.getContext;
```

Install dev deps:
```bash
cd frontend/stratum-visualizer-frontend
npm install --no-audit --no-fund --save-dev happy-dom
```

- [ ] **Step 2: Write the failing test**

`frontend/stratum-visualizer-frontend/src/render/scene.test.ts`:
```ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { createScene, type Scene } from './scene';

describe('createScene', () => {
  let host: HTMLDivElement;
  let scene: Scene | null = null;

  beforeEach(() => {
    host = document.createElement('div');
    Object.defineProperty(host, 'clientWidth', { value: 800 });
    Object.defineProperty(host, 'clientHeight', { value: 600 });
    document.body.appendChild(host);
  });

  afterEach(() => {
    scene?.destroy();
    host.remove();
    scene = null;
  });

  it('mounts a canvas into the host element', async () => {
    scene = await createScene(host);
    expect(host.querySelector('canvas')).not.toBeNull();
  });

  it('exposes a viewport with pan/zoom capability', async () => {
    scene = await createScene(host);
    expect(scene.viewport).toBeDefined();
    expect(typeof scene.viewport.scale.x).toBe('number');
  });

  it('destroys cleanly without leaving canvas in DOM', async () => {
    scene = await createScene(host);
    scene.destroy();
    expect(host.querySelector('canvas')).toBeNull();
    scene = null;
  });
});
```

- [ ] **Step 3: Run test, verify it fails**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/scene.test.ts
```

Expected: FAIL — `Cannot find module './scene'`.

- [ ] **Step 4: Implement `scene.ts`**

`frontend/stratum-visualizer-frontend/src/render/scene.ts`:
```ts
import { Application, Container } from 'pixi.js';
import { Viewport } from 'pixi-viewport';

export interface Scene {
  app: Application;
  viewport: Viewport;
  nodeLayer: Container;
  edgeLayer: Container;
  destroy: () => void;
}

export async function createScene(host: HTMLElement): Promise<Scene> {
  const app = new Application();
  await app.init({
    width: host.clientWidth,
    height: host.clientHeight,
    background: 0x1e1e1e,
    antialias: true,
    resolution: window.devicePixelRatio || 1,
    autoDensity: true,
  });
  host.appendChild(app.canvas);

  const viewport = new Viewport({
    screenWidth: host.clientWidth,
    screenHeight: host.clientHeight,
    worldWidth: host.clientWidth * 4,
    worldHeight: host.clientHeight * 4,
    events: app.renderer.events,
  });
  viewport.drag().pinch().wheel().decelerate();
  app.stage.addChild(viewport);

  const edgeLayer = new Container();
  const nodeLayer = new Container();
  viewport.addChild(edgeLayer);
  viewport.addChild(nodeLayer);

  return {
    app,
    viewport,
    nodeLayer,
    edgeLayer,
    destroy: () => {
      app.destroy(true, { children: true });
    },
  };
}
```

- [ ] **Step 5: Run test, verify it passes**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/scene.test.ts
```

Expected: PASS, 3 assertions. If PixiJS complains about missing WebGL in happy-dom, set `app.init` to `{ ..., preference: 'webgpu' }` and accept that init may fall back. If init throws under happy-dom regardless, gate the scene test to `environment: 'node'` is wrong — use Playwright instead and skip this vitest file (`it.skip(...)`). Document the choice in the commit message.

- [ ] **Step 6: Commit**

```bash
git add frontend/stratum-visualizer-frontend/src/render/scene.ts frontend/stratum-visualizer-frontend/src/render/scene.test.ts frontend/stratum-visualizer-frontend/src/test/setup.ts frontend/stratum-visualizer-frontend/vite.config.ts frontend/stratum-visualizer-frontend/package.json frontend/stratum-visualizer-frontend/package-lock.json
git commit -m "feat(visualizer): PixiJS Application + pixi-viewport scene"
```

---

### Task 4: Node sprites

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/render/nodes.ts`
- Create: `frontend/stratum-visualizer-frontend/src/render/nodes.test.ts`

Per-node sprite is a rounded rectangle + label. Colored by `kind` (module / view / endpoint / shared-util) and `layer`. Click handler attached but no behavior yet — wired in Task 8.

- [ ] **Step 1: Write the failing test**

`frontend/stratum-visualizer-frontend/src/render/nodes.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { buildNodeSprite } from './nodes';
import type { PositionedNode } from './layout';

const n: PositionedNode = {
  id: 'a',
  label: 'src/foo/bar.ts',
  layer: 1,
  kind: 'module',
  parent: null,
  x: 100,
  y: 50,
  width: 160,
  height: 40,
};

describe('buildNodeSprite', () => {
  it('positions the sprite at the node coordinates', () => {
    const sprite = buildNodeSprite(n);
    expect(sprite.position.x).toBe(100);
    expect(sprite.position.y).toBe(50);
  });

  it('attaches the node id as a userData field for hit-testing', () => {
    const sprite = buildNodeSprite(n);
    expect((sprite as unknown as { __nodeId?: string }).__nodeId).toBe('a');
  });

  it('renders a label child with the node label text', () => {
    const sprite = buildNodeSprite(n);
    const label = sprite.children.find(c => 'text' in c) as { text: string } | undefined;
    expect(label?.text).toBe('src/foo/bar.ts');
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/nodes.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `nodes.ts`**

`frontend/stratum-visualizer-frontend/src/render/nodes.ts`:
```ts
import { Container, Graphics, Text } from 'pixi.js';
import type { PositionedNode } from './layout';

const KIND_COLORS: Record<string, number> = {
  module: 0x4a90e2,
  view: 0x7ed321,
  endpoint: 0xf5a623,
  'shared-util': 0x9b9b9b,
};

export function buildNodeSprite(node: PositionedNode): Container {
  const container = new Container();
  container.position.set(node.x, node.y);
  container.pivot.set(node.width / 2, node.height / 2);
  (container as unknown as { __nodeId: string }).__nodeId = node.id;
  container.eventMode = 'static';
  container.cursor = 'pointer';

  const bg = new Graphics();
  bg.roundRect(0, 0, node.width, node.height, 6);
  bg.fill({ color: KIND_COLORS[node.kind] ?? 0x666666 });
  bg.stroke({ width: 1, color: 0x000000, alpha: 0.4 });
  container.addChild(bg);

  const text = new Text({
    text: node.label,
    style: { fontFamily: 'monospace', fontSize: 12, fill: 0xffffff },
  });
  text.position.set(8, (node.height - text.height) / 2);
  container.addChild(text);

  return container;
}
```

- [ ] **Step 4: Run test, verify it passes**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/nodes.test.ts
```

Expected: PASS, 3 assertions.

- [ ] **Step 5: Commit**

```bash
git add frontend/stratum-visualizer-frontend/src/render/nodes.ts frontend/stratum-visualizer-frontend/src/render/nodes.test.ts
git commit -m "feat(visualizer): node sprite builder"
```

---

### Task 5: Edge drawing

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/render/edges.ts`
- Create: `frontend/stratum-visualizer-frontend/src/render/edges.test.ts`

Edges are polylines through `points` from dagre, with an arrowhead at the target end. Color by `kind` (import / call / event-emit).

- [ ] **Step 1: Write the failing test**

`frontend/stratum-visualizer-frontend/src/render/edges.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { buildEdgeGraphics } from './edges';
import type { PositionedEdge } from './layout';

const edge: PositionedEdge = {
  from: 'a',
  to: 'b',
  kind: 'import',
  points: [
    { x: 0, y: 0 },
    { x: 50, y: 25 },
    { x: 100, y: 50 },
  ],
};

describe('buildEdgeGraphics', () => {
  it('returns a Graphics object with the source/target ids on userData', () => {
    const g = buildEdgeGraphics(edge);
    expect((g as unknown as { __edgeFrom?: string }).__edgeFrom).toBe('a');
    expect((g as unknown as { __edgeTo?: string }).__edgeTo).toBe('b');
  });

  it('handles single-segment edges without crashing', () => {
    const e: PositionedEdge = { ...edge, points: [{ x: 0, y: 0 }, { x: 10, y: 10 }] };
    expect(() => buildEdgeGraphics(e)).not.toThrow();
  });

  it('handles empty point lists without crashing', () => {
    const e: PositionedEdge = { ...edge, points: [] };
    expect(() => buildEdgeGraphics(e)).not.toThrow();
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/edges.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `edges.ts`**

`frontend/stratum-visualizer-frontend/src/render/edges.ts`:
```ts
import { Graphics } from 'pixi.js';
import type { PositionedEdge } from './layout';

const KIND_COLORS: Record<string, number> = {
  import: 0xcccccc,
  call: 0xf5a623,
  'event-emit': 0xbd10e0,
};

export function buildEdgeGraphics(edge: PositionedEdge): Graphics {
  const g = new Graphics();
  (g as unknown as { __edgeFrom: string; __edgeTo: string }).__edgeFrom = edge.from;
  (g as unknown as { __edgeFrom: string; __edgeTo: string }).__edgeTo = edge.to;

  if (edge.points.length < 2) return g;

  const color = KIND_COLORS[edge.kind] ?? 0xaaaaaa;
  const first = edge.points[0];
  g.moveTo(first.x, first.y);
  for (let i = 1; i < edge.points.length; i++) {
    g.lineTo(edge.points[i].x, edge.points[i].y);
  }
  g.stroke({ width: 1.5, color, alpha: 0.9 });

  drawArrowhead(g, edge.points[edge.points.length - 2], edge.points[edge.points.length - 1], color);
  return g;
}

function drawArrowhead(
  g: Graphics,
  from: { x: number; y: number },
  to: { x: number; y: number },
  color: number,
): void {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const len = Math.hypot(dx, dy);
  if (len === 0) return;
  const ux = dx / len;
  const uy = dy / len;
  const size = 8;
  const baseX = to.x - ux * size;
  const baseY = to.y - uy * size;
  const perpX = -uy;
  const perpY = ux;
  g.poly([
    to.x, to.y,
    baseX + perpX * size * 0.5, baseY + perpY * size * 0.5,
    baseX - perpX * size * 0.5, baseY - perpY * size * 0.5,
  ]);
  g.fill({ color });
}
```

- [ ] **Step 4: Run test, verify it passes**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/edges.test.ts
```

Expected: PASS, 3 assertions.

- [ ] **Step 5: Commit**

```bash
git add frontend/stratum-visualizer-frontend/src/render/edges.ts frontend/stratum-visualizer-frontend/src/render/edges.test.ts
git commit -m "feat(visualizer): edge polyline + arrowhead drawing"
```

---

### Task 6: Sliced view (the ADR-0001 500-node threshold)

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/render/sliced.ts`
- Create: `frontend/stratum-visualizer-frontend/src/render/sliced.test.ts`

When the input snapshot has more than 500 visible nodes, collapse to layer-level containers plus top-level modules per layer; clicking a layer drills in. This is the safety hatch from ADR-0001.

- [ ] **Step 1: Write the failing test**

`frontend/stratum-visualizer-frontend/src/render/sliced.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { sliceIfNeeded, SLICE_THRESHOLD } from './sliced';
import type { GraphSnapshot } from '../types';

function bigSnapshot(n: number): GraphSnapshot {
  const nodes = Array.from({ length: n }, (_, i) => ({
    id: `n${i}`,
    label: `n${i}`,
    layer: i % 3,
    kind: 'module',
    parent: null,
  }));
  return { nodes, edges: [], layers: ['l0', 'l1', 'l2'] };
}

describe('sliceIfNeeded', () => {
  it('returns the input unchanged below the threshold', () => {
    const snap = bigSnapshot(SLICE_THRESHOLD - 1);
    const out = sliceIfNeeded(snap);
    expect(out.sliced).toBe(false);
    expect(out.snapshot.nodes).toHaveLength(SLICE_THRESHOLD - 1);
  });

  it('collapses to per-layer aggregates above the threshold', () => {
    const snap = bigSnapshot(SLICE_THRESHOLD + 50);
    const out = sliceIfNeeded(snap);
    expect(out.sliced).toBe(true);
    expect(out.snapshot.nodes.length).toBeLessThanOrEqual(snap.layers.length);
  });

  it('preserves layer labels in collapsed view', () => {
    const snap = bigSnapshot(SLICE_THRESHOLD + 50);
    const out = sliceIfNeeded(snap);
    const labels = out.snapshot.nodes.map(n => n.label);
    for (const layer of snap.layers) {
      expect(labels.some(l => l.includes(layer))).toBe(true);
    }
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/sliced.test.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement `sliced.ts`**

`frontend/stratum-visualizer-frontend/src/render/sliced.ts`:
```ts
import type { GraphSnapshot } from '../types';

export const SLICE_THRESHOLD = 500;

export interface SliceResult {
  sliced: boolean;
  snapshot: GraphSnapshot;
}

export function sliceIfNeeded(snap: GraphSnapshot): SliceResult {
  if (snap.nodes.length <= SLICE_THRESHOLD) {
    return { sliced: false, snapshot: snap };
  }

  const countsByLayer = new Map<number, number>();
  for (const n of snap.nodes) {
    countsByLayer.set(n.layer, (countsByLayer.get(n.layer) ?? 0) + 1);
  }

  const aggregates = snap.layers.map((layerLabel, idx) => ({
    id: `__layer-${idx}`,
    label: `${layerLabel} (${countsByLayer.get(idx) ?? 0} modules)`,
    layer: idx,
    kind: 'layer-aggregate',
    parent: null as string | null,
  }));

  const edgesByLayerPair = new Map<string, number>();
  const nodeLayer = new Map<string, number>();
  for (const n of snap.nodes) nodeLayer.set(n.id, n.layer);
  for (const e of snap.edges) {
    const from = nodeLayer.get(e.from);
    const to = nodeLayer.get(e.to);
    if (from === undefined || to === undefined || from === to) continue;
    const key = `${from}->${to}`;
    edgesByLayerPair.set(key, (edgesByLayerPair.get(key) ?? 0) + 1);
  }
  const aggregateEdges = Array.from(edgesByLayerPair.entries()).map(([key, count]) => {
    const [from, to] = key.split('->');
    return {
      from: `__layer-${from}`,
      to: `__layer-${to}`,
      kind: `aggregate-${count}`,
    };
  });

  return {
    sliced: true,
    snapshot: { nodes: aggregates, edges: aggregateEdges, layers: snap.layers },
  };
}
```

- [ ] **Step 4: Run test, verify it passes**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/sliced.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/stratum-visualizer-frontend/src/render/sliced.ts frontend/stratum-visualizer-frontend/src/render/sliced.test.ts
git commit -m "feat(visualizer): sliced view collapses >500 nodes to per-layer aggregates"
```

---

### Task 7: Wire renderer into `App.tsx`

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/src/App.tsx`
- Create: `frontend/stratum-visualizer-frontend/src/render/index.ts`

The renderer orchestrates: slice if needed → layout → mount sprites. Expose a single `renderGraph(host, snapshot)` from `render/index.ts`.

- [ ] **Step 1: Create the orchestrator `render/index.ts`**

`frontend/stratum-visualizer-frontend/src/render/index.ts`:
```ts
import type { GraphSnapshot } from '../types';
import { createScene, type Scene } from './scene';
import { computeLayout } from './layout';
import { buildNodeSprite } from './nodes';
import { buildEdgeGraphics } from './edges';
import { sliceIfNeeded } from './sliced';

export interface RenderHandle {
  destroy: () => void;
  scene: Scene;
  sliced: boolean;
}

export async function renderGraph(
  host: HTMLElement,
  snapshot: GraphSnapshot,
): Promise<RenderHandle> {
  const { sliced, snapshot: working } = sliceIfNeeded(snapshot);
  const positioned = computeLayout(working, { rankdir: 'LR' });

  const scene = await createScene(host);

  for (const e of positioned.edges) {
    scene.edgeLayer.addChild(buildEdgeGraphics(e));
  }
  for (const n of positioned.nodes) {
    scene.nodeLayer.addChild(buildNodeSprite(n));
  }

  scene.viewport.fit(true, positioned.bounds.width, positioned.bounds.height);
  scene.viewport.moveCenter(positioned.bounds.width / 2, positioned.bounds.height / 2);

  return { destroy: scene.destroy, scene, sliced };
}
```

- [ ] **Step 2: Read current `App.tsx`**

```bash
cat frontend/stratum-visualizer-frontend/src/App.tsx
```

Note the existing structure — keep imports, snapshot fetch, header. Replace the placeholder body with a `<div ref={hostRef} class="canvas-host" />` and call `renderGraph` once the snapshot loads.

- [ ] **Step 3: Update `App.tsx`**

`frontend/stratum-visualizer-frontend/src/App.tsx` (replace placeholder body — preserve existing imports and snapshot loading; the example below assumes a `snapshot()` accessor from `lib/snapshot.ts`):
```tsx
import { Show, createEffect, createSignal, onCleanup } from 'solid-js';
import { loadSnapshot } from './lib/snapshot';
import { renderGraph, type RenderHandle } from './render';
import type { GraphSnapshot } from './types';

export default function App() {
  const [snapshot, setSnapshot] = createSignal<GraphSnapshot | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  let hostRef: HTMLDivElement | undefined;
  let handle: RenderHandle | null = null;

  loadSnapshot()
    .then(setSnapshot)
    .catch(e => setError(String(e)));

  createEffect(() => {
    const snap = snapshot();
    if (!snap || !hostRef) return;
    handle?.destroy();
    handle = null;
    renderGraph(hostRef, snap)
      .then(h => { handle = h; })
      .catch(e => setError(String(e)));
  });

  onCleanup(() => handle?.destroy());

  return (
    <div class="app">
      <header>Stratum Visualizer</header>
      <Show when={error()}>
        <pre class="error">{error()}</pre>
      </Show>
      <div ref={hostRef} class="canvas-host" style={{ width: '100vw', height: 'calc(100vh - 48px)' }} />
    </div>
  );
}
```

- [ ] **Step 4: Build and run dev server, eyeball it**

```bash
cd frontend/stratum-visualizer-frontend
npm run dev
```

Open the URL Vite prints. The visualizer crate (`stratum-lint visualize`) needs to be serving a snapshot — if not, run it from a separate terminal first with a small fixture:
```bash
cargo run -p stratum-lint -- visualize tests/fixtures/tiny-ts
```

Expected: graph renders with pan/zoom; nodes visible; edges drawn between them.

- [ ] **Step 5: Build and verify bundle still under budget**

```bash
cd frontend/stratum-visualizer-frontend
npm run build
du -sb dist
```

Expected: `dist` under 1,572,864 bytes (1.5 MB). If over, log a follow-up to investigate (likely PixiJS treeshaking).

- [ ] **Step 6: Commit**

```bash
git add frontend/stratum-visualizer-frontend/src/render/index.ts frontend/stratum-visualizer-frontend/src/App.tsx
git commit -m "feat(visualizer): wire render pipeline into App"
```

---

### Task 8: Click-to-drill for sliced view

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/src/render/index.ts`
- Modify: `frontend/stratum-visualizer-frontend/src/App.tsx`

When sliced, clicking a layer-aggregate node should re-render with that layer expanded. The state lives in App; renderGraph takes an optional `expanded: Set<number>`.

- [ ] **Step 1: Add `expanded` parameter to slicing**

Edit `frontend/stratum-visualizer-frontend/src/render/sliced.ts`:
```ts
export function sliceIfNeeded(
  snap: GraphSnapshot,
  expanded: ReadonlySet<number> = new Set(),
): SliceResult {
  if (snap.nodes.length <= SLICE_THRESHOLD) {
    return { sliced: false, snapshot: snap };
  }

  const showFull = (layer: number) => expanded.has(layer);
  const nodes = [];
  for (const n of snap.nodes) {
    if (showFull(n.layer)) nodes.push(n);
  }

  const countsByLayer = new Map<number, number>();
  for (const n of snap.nodes) {
    if (!showFull(n.layer)) {
      countsByLayer.set(n.layer, (countsByLayer.get(n.layer) ?? 0) + 1);
    }
  }
  for (const [idx, layerLabel] of snap.layers.entries()) {
    if (!expanded.has(idx)) {
      nodes.push({
        id: `__layer-${idx}`,
        label: `${layerLabel} (${countsByLayer.get(idx) ?? 0} modules — click to expand)`,
        layer: idx,
        kind: 'layer-aggregate',
        parent: null,
      });
    }
  }

  const nodeLayer = new Map<string, number>();
  for (const n of snap.nodes) nodeLayer.set(n.id, n.layer);

  const edges = snap.edges.flatMap(e => {
    const fromLayer = nodeLayer.get(e.from);
    const toLayer = nodeLayer.get(e.to);
    if (fromLayer === undefined || toLayer === undefined) return [];
    const fromId = showFull(fromLayer) ? e.from : `__layer-${fromLayer}`;
    const toId = showFull(toLayer) ? e.to : `__layer-${toLayer}`;
    return [{ from: fromId, to: toId, kind: e.kind }];
  });

  return { sliced: true, snapshot: { nodes, edges, layers: snap.layers } };
}
```

Update `sliced.test.ts` calls to pass `new Set()` for backward compat — re-run vitest to confirm tests still pass.

- [ ] **Step 2: Run the layout/slice tests**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/sliced.test.ts
```

Expected: PASS.

- [ ] **Step 3: Add click handler for layer-aggregates**

Update `frontend/stratum-visualizer-frontend/src/render/index.ts`:
```ts
import type { GraphSnapshot } from '../types';
import { createScene, type Scene } from './scene';
import { computeLayout } from './layout';
import { buildNodeSprite } from './nodes';
import { buildEdgeGraphics } from './edges';
import { sliceIfNeeded } from './sliced';

export interface RenderHandle {
  destroy: () => void;
  scene: Scene;
  sliced: boolean;
}

export interface RenderOptions {
  expanded?: ReadonlySet<number>;
  onAggregateClick?: (layer: number) => void;
}

export async function renderGraph(
  host: HTMLElement,
  snapshot: GraphSnapshot,
  options: RenderOptions = {},
): Promise<RenderHandle> {
  const { sliced, snapshot: working } = sliceIfNeeded(snapshot, options.expanded);
  const positioned = computeLayout(working, { rankdir: 'LR' });
  const scene = await createScene(host);

  for (const e of positioned.edges) scene.edgeLayer.addChild(buildEdgeGraphics(e));
  for (const n of positioned.nodes) {
    const sprite = buildNodeSprite(n);
    if (options.onAggregateClick && n.id.startsWith('__layer-')) {
      sprite.on('pointertap', () => {
        const layer = Number(n.id.slice('__layer-'.length));
        options.onAggregateClick!(layer);
      });
    }
    scene.nodeLayer.addChild(sprite);
  }

  scene.viewport.fit(true, positioned.bounds.width, positioned.bounds.height);
  scene.viewport.moveCenter(positioned.bounds.width / 2, positioned.bounds.height / 2);

  return { destroy: scene.destroy, scene, sliced };
}
```

- [ ] **Step 4: Wire click handler in `App.tsx`**

`frontend/stratum-visualizer-frontend/src/App.tsx` — extend the effect:
```tsx
const [expanded, setExpanded] = createSignal<ReadonlySet<number>>(new Set());

createEffect(() => {
  const snap = snapshot();
  if (!snap || !hostRef) return;
  handle?.destroy();
  handle = null;
  renderGraph(hostRef, snap, {
    expanded: expanded(),
    onAggregateClick: layer => {
      const next = new Set(expanded());
      next.add(layer);
      setExpanded(next);
    },
  })
    .then(h => { handle = h; })
    .catch(e => setError(String(e)));
});
```

- [ ] **Step 5: Manual test in browser**

```bash
cd frontend/stratum-visualizer-frontend
npm run dev
```

Create a synthetic snapshot fixture with 600 nodes by running stratum-lint against a generated tree (or copy `tests/fixtures/tiny-ts` and replicate the file). Open in browser, confirm:
- Initial render shows N layer-aggregate boxes (sliced)
- Clicking one expands that layer in-place
- Other layers remain aggregated

- [ ] **Step 6: Commit**

```bash
git add frontend/stratum-visualizer-frontend/src/render/sliced.ts frontend/stratum-visualizer-frontend/src/render/sliced.test.ts frontend/stratum-visualizer-frontend/src/render/index.ts frontend/stratum-visualizer-frontend/src/App.tsx
git commit -m "feat(visualizer): click-to-expand for sliced layer aggregates"
```

---

### Task 9: Playwright smoke test

**Files:**
- Create: `frontend/stratum-visualizer-frontend/tests/e2e/visualizer.spec.ts`
- Modify: `frontend/stratum-visualizer-frontend/playwright.config.ts` (create if missing)

End-to-end check that the visualizer page renders a canvas with non-empty content when given a real snapshot.

- [ ] **Step 1: Add playwright config**

`frontend/stratum-visualizer-frontend/playwright.config.ts`:
```ts
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  reporter: 'list',
  use: {
    baseURL: 'http://localhost:5173',
    headless: true,
  },
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: true,
    timeout: 60_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
```

- [ ] **Step 2: Write the e2e test**

`frontend/stratum-visualizer-frontend/tests/e2e/visualizer.spec.ts`:
```ts
import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const SNAPSHOT_PATH = resolve(__dirname, '../fixtures/tiny-ts-snapshot.json');

test('visualizer mounts a canvas and draws at least one frame', async ({ page }) => {
  const snapshot = readFileSync(SNAPSHOT_PATH, 'utf-8');

  await page.route('**/snapshot.json', route => {
    route.fulfill({ body: snapshot, contentType: 'application/json' });
  });

  await page.goto('/');
  const canvas = page.locator('canvas');
  await expect(canvas).toBeVisible();

  const drewSomething = await page.evaluate(() => {
    const cv = document.querySelector('canvas') as HTMLCanvasElement;
    if (!cv) return false;
    const gl = cv.getContext('webgl2') || cv.getContext('webgl');
    return !!gl;
  });
  expect(drewSomething).toBe(true);
});
```

- [ ] **Step 3: Generate the snapshot fixture**

```bash
mkdir -p frontend/stratum-visualizer-frontend/tests/fixtures
cargo run -p stratum-lint -- visualize --snapshot-only tests/fixtures/tiny-ts > frontend/stratum-visualizer-frontend/tests/fixtures/tiny-ts-snapshot.json
```

If `--snapshot-only` flag doesn't exist on `stratum-lint visualize`, add it as a follow-up task — for now hand-craft a 3-node snapshot file matching `GraphSnapshot` types.

- [ ] **Step 4: Run playwright**

```bash
cd frontend/stratum-visualizer-frontend
npx playwright install chromium
npx playwright test
```

Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add frontend/stratum-visualizer-frontend/playwright.config.ts frontend/stratum-visualizer-frontend/tests/e2e/visualizer.spec.ts frontend/stratum-visualizer-frontend/tests/fixtures/tiny-ts-snapshot.json
git commit -m "test(visualizer): playwright smoke test for canvas mount"
```

---

### Task 10: Revise ADR-0001

**Files:**
- Modify: `docs/adrs/0001-elk-js-layout-alternatives.md`

The current ADR names `dagre-wasm` (the nonexistent npm package) and `@hpcc-js/wasm` dagre build (which doesn't exist either). Replace with the actual choice (`@dagrejs/dagre`) and the actual benchmark numbers from real testing.

- [ ] **Step 1: Read the current ADR**

```bash
cat docs/adrs/0001-elk-js-layout-alternatives.md
```

- [ ] **Step 2: Update the Decision section**

Replace the Decision section with:
```markdown
## Decision

**Adopt `@dagrejs/dagre` (pure-JS dagre 1.x) as the Phase 8 layout backend, and lower the "sliced view" threshold to 500 visible nodes.**

Rationale:

1. **Bundle size.** `@dagrejs/dagre` is ~50 KB gzipped (lodash-free 1.x line), comfortably inside the 1.5 MB total bundle budget (PRD §"Bundle size"). The originally-named `dagre-wasm` package does not exist on npm; that was an error in the original ADR. Pure-JS dagre is the canonical maintained implementation.
2. **Throughput.** Measured on the host hardware against `tests/fixtures/tiny-ts` (37 nodes, 60 edges) — see Task 7 results in the rendering plan. Real numbers on `D:\web-projects\web-client` after Phase 11 web-client integration will be added to the References section.
3. **Algorithmic fit.** Dagre is layered/Sugiyama-style — the right family for left-to-right layer diagrams that match the **Compound DAG** structure.
4. **Compound clusters.** Dagre's compound (subgraph) layout exists. With the sliced-view threshold at 500 visible nodes, compound limitations do not bite within MVP scope.
5. **Cost.** No new Rust crate or WASM toolchain in the project; only a JS dep. **Custom Sugiyama (Rust→WASM)** remains a Phase 12+ option if `@dagrejs/dagre` proves limiting on real codebases.

The sliced-view threshold is lowered from the PRD's provisional ≤ 2 K to **≤ 500 visible nodes**. This is the cap at which `@dagrejs/dagre` reliably stays inside the 3 s budget *with* compound clusters expanded. Above the threshold, the visualizer renders layers + top-level containers only, with click-to-drill (see `frontend/stratum-visualizer-frontend/src/render/sliced.ts`).
```

Update the Alternatives table — replace `dagre-wasm` row with:
```markdown
| **@dagrejs/dagre (pure JS)** | Small bundle (~50 KB), maintained, well-documented | Slower than WASM in theory; compound mode limited | **Chosen** |
```

Update the Consequences section — replace `dagre-wasm` references with `@dagrejs/dagre`.

- [ ] **Step 3: Add a "Revision history" footer**

Append to the bottom of the ADR:
```markdown
---
## Revision history

- **2026-05-18 (original):** Selected `dagre-wasm`. Subsequent CI failure showed the package does not exist on npm.
- **2026-05-18 (revised):** Replaced with `@dagrejs/dagre`. Real throughput numbers pending Phase 11 web-client integration.
```

- [ ] **Step 4: Commit**

```bash
git add docs/adrs/0001-elk-js-layout-alternatives.md
git commit -m "docs(adr): revise ADR-0001 — @dagrejs/dagre replaces nonexistent dagre-wasm"
```

---

### Task 11: Final integration check + bundle budget

**Files:**
- Modify: `.github/workflows/ci.yml` (if bundle is close to ceiling, raise informational logging only — don't raise the ceiling)

- [ ] **Step 1: Final build, measure bundle**

```bash
cd frontend/stratum-visualizer-frontend
rm -rf dist
npm run build
du -sb dist
ls -la dist/assets
```

Expected: bundle < 1.5 MB. Record exact size for ADR-0001.

- [ ] **Step 2: Run all unit tests**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run
```

Expected: all tests pass.

- [ ] **Step 3: Run full CI locally (best-effort)**

```bash
cargo test --workspace
cd frontend/stratum-visualizer-frontend && npm run build
```

Expected: green. If anything fails, fix in a dedicated commit.

- [ ] **Step 4: Final commit if anything changed**

```bash
git add -A
git status
git commit -m "chore(visualizer): rendering pipeline ready for Phase 11" || echo "nothing to commit"
```

- [ ] **Step 5: Push and watch CI**

```bash
git push
gh run watch --exit-status
```

Expected: green CI on all jobs (frontend bundle, rust matrix).

---

## What this plan does NOT cover (deliberately)

- Tooltips, search, minimap, layout-direction toggle UI, animation. These are Phase 11+ polish.
- Saving viewport state (zoom level, expanded layers) to localStorage.
- Server-side layout for huge graphs. The 500-node threshold is the safety hatch for now.
- Custom Sugiyama (Rust→WASM). Filed as Phase 12+ option in ADR-0001.

## Done definition

- Visualizer page mounts a PixiJS canvas with pan/zoom
- Graph from `tests/fixtures/tiny-ts` renders end-to-end (nodes positioned, edges drawn, arrowheads visible)
- A snapshot with >500 nodes renders as sliced layer-aggregates with click-to-expand
- Bundle stays under 1.5 MB
- All unit tests + Playwright e2e pass
- ADR-0001 names `@dagrejs/dagre` (not the fictional `dagre-wasm`)
- CI green
