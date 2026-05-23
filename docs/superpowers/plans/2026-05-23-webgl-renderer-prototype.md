# WebGL renderer prototype — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a feature-flagged WebGL renderer (`<GraphWebGL/>`) alongside the existing SVG `<Graph/>`, both consuming the same `Scene` from `render/layout.ts`. Manual A/B trace on web-client decides go/no-go.

**Architecture:** PixiJS v8 batches geometry (lanes, containers, modules, edges). Text labels render in an HTML overlay synced to viewport. Solid signal `renderer: 'svg' | 'webgl'` swaps components via `<Show>`. Layout pipeline (`adapt → computeLayout → Scene`) untouched.

**Tech Stack:** PixiJS 8.x, pixi-viewport 6.x, SolidJS 1.8.x, TypeScript 5.4.x. Vitest for the two pure modules (color conversion, spatial hit-test); no test for Pixi side-effect code (validate manually in Tasks 11 + 12).

**Spec:** `docs/superpowers/specs/2026-05-23-webgl-renderer-prototype-design.md`

---

### Task 1: Add dependencies (pixi.js + pixi-viewport)

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/package.json`

- [ ] **Step 1: Update `dependencies`**

Open `frontend/stratum-visualizer-frontend/package.json`. Replace the `dependencies` block to add the two new entries:

```json
  "dependencies": {
    "solid-js": "^1.8.0",
    "pixi.js": "^8.0.0",
    "pixi-viewport": "^6.0.2"
  },
```

`pixi-viewport@^6.0.2` is the first 6.x version supporting Pixi v8 peer dep. If npm install errors with `ERESOLVE` mentioning peer pixi.js v7 — try `^6.0.3`. If that also fails, see Appendix B for a 50-line manual pan/zoom fallback.

- [ ] **Step 2: Install**

Run:
```bash
cd frontend/stratum-visualizer-frontend
npm install --no-audit --no-fund
```

Expected: no errors. `node_modules/pixi.js/package.json` exists; `node_modules/pixi-viewport/package.json` exists.

- [ ] **Step 3: Verify build still works**

```bash
npm run build
```

Expected: `dist/` rebuilt, no TypeScript errors. Bundle delta is informational only — the prototype isn't subject to the 1.5MB CI budget because both renderers coexist.

- [ ] **Step 4: Commit**

```bash
git add frontend/stratum-visualizer-frontend/package.json frontend/stratum-visualizer-frontend/package-lock.json
git commit -m "build(visualizer): add pixi.js + pixi-viewport for WebGL prototype"
```

---

### Task 2: Pure color module — oklch → linear sRGB

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/render/webgl/styles.ts`
- Create: `frontend/stratum-visualizer-frontend/src/render/webgl/styles.test.ts`

- [ ] **Step 1: Write failing test for `oklchToRgbHex`**

Create `src/render/webgl/styles.test.ts`:

```typescript
import { describe, expect, it } from 'vitest'
import { oklchToRgbHex, severityFill, layerHueToHex } from './styles'

describe('oklchToRgbHex', () => {
  it('converts pure white (oklch(100% 0 0)) to 0xffffff', () => {
    expect(oklchToRgbHex(100, 0, 0)).toBe(0xffffff)
  })

  it('converts pure black (oklch(0% 0 0)) to 0x000000', () => {
    expect(oklchToRgbHex(0, 0, 0)).toBe(0x000000)
  })

  it('produces a 24-bit integer for arbitrary oklch input', () => {
    const v = oklchToRgbHex(64, 0.18, 25)
    expect(v).toBeGreaterThanOrEqual(0)
    expect(v).toBeLessThanOrEqual(0xffffff)
    expect(Number.isInteger(v)).toBe(true)
  })

  it('is deterministic — same input gives same output', () => {
    expect(oklchToRgbHex(64, 0.18, 25)).toBe(oklchToRgbHex(64, 0.18, 25))
  })
})

describe('severityFill', () => {
  it('returns distinct hex values for error / warning / info', () => {
    const err = severityFill('error')
    const warn = severityFill('warning')
    const info = severityFill('info')
    expect(err).not.toBe(warn)
    expect(warn).not.toBe(info)
    expect(err).not.toBe(info)
  })
})

describe('layerHueToHex', () => {
  it('maps hue to a deterministic hex value', () => {
    expect(layerHueToHex(120)).toBe(layerHueToHex(120))
  })
})
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
cd frontend/stratum-visualizer-frontend
npx vitest run src/render/webgl/styles.test.ts
```

Expected: FAIL with "Cannot find module './styles'".

- [ ] **Step 3: Write `styles.ts`**

Create `src/render/webgl/styles.ts`:

```typescript
// oklch → linear sRGB → 24-bit hex for Pixi tint/fill.
//
// Inputs are the same as CSS oklch():
//   l: 0..100 (percent)
//   c: 0..0.4 typical
//   h: 0..360 degrees
//
// Round-trip target is "close enough for prototype" — exact CSS color match
// is not required because the renderer is opt-in via a feature flag.

const oklabToLinear = (l: number, a: number, b: number): [number, number, number] => {
  const l_ = l + 0.3963377774 * a + 0.2158037573 * b
  const m_ = l - 0.1055613458 * a - 0.0638541728 * b
  const s_ = l - 0.0894841775 * a - 1.291485548 * b

  const lc = l_ * l_ * l_
  const mc = m_ * m_ * m_
  const sc = s_ * s_ * s_

  return [
    +4.0767416621 * lc - 3.3077115913 * mc + 0.2309699292 * sc,
    -1.2684380046 * lc + 2.6097574011 * mc - 0.3413193965 * sc,
    -0.0041960863 * lc - 0.7034186147 * mc + 1.707614701 * sc,
  ]
}

const linearToSrgb = (v: number): number => {
  if (v <= 0) return 0
  if (v >= 1) return 1
  return v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(v, 1 / 2.4) - 0.055
}

export const oklchToRgbHex = (l: number, c: number, h: number): number => {
  const hRad = (h * Math.PI) / 180
  const a = c * Math.cos(hRad)
  const b = c * Math.sin(hRad)
  const [rL, gL, bL] = oklabToLinear(l / 100, a, b)
  const r = Math.round(linearToSrgb(rL) * 255)
  const g = Math.round(linearToSrgb(gL) * 255)
  const bv = Math.round(linearToSrgb(bL) * 255)
  return (r << 16) | (g << 8) | bv
}

export const layerHueToHex = (hue: number, l = 60, c = 0.1): number =>
  oklchToRgbHex(l, c, hue)

const SEVERITY_FILL = {
  error: oklchToRgbHex(64, 0.18, 25),
  warning: oklchToRgbHex(78, 0.14, 70),
  info: oklchToRgbHex(70, 0.1, 230),
} as const

export type Severity = keyof typeof SEVERITY_FILL

export const severityFill = (s: Severity): number => SEVERITY_FILL[s]

export const severityGlow = severityFill

export const MODULE_FILL = oklchToRgbHex(18, 0.02, 250)
export const MODULE_STROKE = oklchToRgbHex(70, 0.02, 250)
export const CONTAINER_FILL = oklchToRgbHex(13, 0.02, 250)
export const LANE_FILL = oklchToRgbHex(11, 0.01, 250)
export const TEXT_COLOR = oklchToRgbHex(88, 0.02, 250)
```

- [ ] **Step 4: Run test — expect PASS**

```bash
npx vitest run src/render/webgl/styles.test.ts
```

Expected: all 6 assertions pass.

- [ ] **Step 5: Commit**

```bash
git add src/render/webgl/styles.ts src/render/webgl/styles.test.ts
git commit -m "feat(viz/webgl): oklch→sRGB color conversion module"
```

---

### Task 3: Pure spatial hit-test (bucket grid)

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/render/webgl/hitTest.ts`
- Create: `frontend/stratum-visualizer-frontend/src/render/webgl/hitTest.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/render/webgl/hitTest.test.ts`:

```typescript
import { describe, expect, it } from 'vitest'
import { buildHitTest, HitTestIndex } from './hitTest'
import type { Scene } from '../layout'
import type { DesignModule } from '../design'

const mkMod = (id: string): DesignModule => ({
  id,
  label: id,
  path: id,
  kind: 'regular',
  loc: 10,
  container: 'c1',
  parentId: null,
  childIds: [],
  layer: 'app',
  stage: 0,
  severity: null,
  violations: [],
  descError: 0,
  descWarn: 0,
})

const mkScene = (): Scene => ({
  lanes: [],
  containers: [],
  edges: [],
  width: 1000,
  height: 1000,
  modulePos: {
    a: { x: 10, y: 10, w: 30, h: 30, cx: 25, cy: 25, mod: mkMod('a'), leaf: true, depth: 0, childIds: [] },
    b: { x: 100, y: 100, w: 40, h: 40, cx: 120, cy: 120, mod: mkMod('b'), leaf: true, depth: 0, childIds: [] },
    c: { x: 200, y: 200, w: 20, h: 20, cx: 210, cy: 210, mod: mkMod('c'), leaf: true, depth: 0, childIds: [] },
  },
})

describe('buildHitTest', () => {
  let idx: HitTestIndex
  it('returns an index', () => {
    idx = buildHitTest(mkScene())
    expect(idx).toBeDefined()
  })

  it('finds a module at its centre', () => {
    expect(idx.queryPoint(25, 25)).toBe('a')
    expect(idx.queryPoint(120, 120)).toBe('b')
  })

  it('returns null outside any module', () => {
    expect(idx.queryPoint(1000, 1000)).toBeNull()
    expect(idx.queryPoint(70, 70)).toBeNull()
  })

  it('finds modules at their edges (inclusive of x/y, exclusive of x+w/y+h)', () => {
    expect(idx.queryPoint(10, 10)).toBe('a')
    expect(idx.queryPoint(39.9, 39.9)).toBe('a')
    expect(idx.queryPoint(40, 40)).toBeNull()
  })

  it('returns the innermost (smallest) overlapping module when multiple match', () => {
    const overlap: Scene = {
      ...mkScene(),
      modulePos: {
        big: { x: 0, y: 0, w: 100, h: 100, cx: 50, cy: 50, mod: mkMod('big'), leaf: false, depth: 0, childIds: ['small'] },
        small: { x: 40, y: 40, w: 20, h: 20, cx: 50, cy: 50, mod: mkMod('small'), leaf: true, depth: 1, childIds: [] },
      },
    }
    const i = buildHitTest(overlap)
    expect(i.queryPoint(50, 50)).toBe('small')
    expect(i.queryPoint(5, 5)).toBe('big')
  })
})
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
npx vitest run src/render/webgl/hitTest.test.ts
```

Expected: FAIL with "Cannot find module './hitTest'".

- [ ] **Step 3: Write `hitTest.ts`**

Create `src/render/webgl/hitTest.ts`:

```typescript
import type { Scene, ModulePos } from '../layout'

export interface HitTestIndex {
  queryPoint(x: number, y: number): string | null
}

interface CellEntry {
  id: string
  x: number
  y: number
  w: number
  h: number
  area: number
}

const DEFAULT_CELL = 128

export const buildHitTest = (scene: Scene, cellSize = DEFAULT_CELL): HitTestIndex => {
  const cells = new Map<number, CellEntry[]>()
  const cols = Math.max(1, Math.ceil(scene.width / cellSize))

  const keyOf = (cx: number, cy: number): number => cy * cols + cx

  const insert = (id: string, p: ModulePos) => {
    const entry: CellEntry = { id, x: p.x, y: p.y, w: p.w, h: p.h, area: p.w * p.h }
    const cx0 = Math.floor(p.x / cellSize)
    const cy0 = Math.floor(p.y / cellSize)
    const cx1 = Math.floor((p.x + p.w - 0.001) / cellSize)
    const cy1 = Math.floor((p.y + p.h - 0.001) / cellSize)
    for (let cy = cy0; cy <= cy1; cy++) {
      for (let cx = cx0; cx <= cx1; cx++) {
        const k = keyOf(cx, cy)
        let arr = cells.get(k)
        if (!arr) {
          arr = []
          cells.set(k, arr)
        }
        arr.push(entry)
      }
    }
  }

  for (const [id, p] of Object.entries(scene.modulePos)) insert(id, p)

  const queryPoint = (x: number, y: number): string | null => {
    const cx = Math.floor(x / cellSize)
    const cy = Math.floor(y / cellSize)
    const k = keyOf(cx, cy)
    const arr = cells.get(k)
    if (!arr) return null
    let bestId: string | null = null
    let bestArea = Infinity
    for (const e of arr) {
      if (x >= e.x && x < e.x + e.w && y >= e.y && y < e.y + e.h) {
        if (e.area < bestArea) {
          bestArea = e.area
          bestId = e.id
        }
      }
    }
    return bestId
  }

  return { queryPoint }
}
```

- [ ] **Step 4: Run test — expect PASS**

```bash
npx vitest run src/render/webgl/hitTest.test.ts
```

Expected: all 5 assertions pass.

- [ ] **Step 5: Commit**

```bash
git add src/render/webgl/hitTest.ts src/render/webgl/hitTest.test.ts
git commit -m "feat(viz/webgl): bucket-grid hit-test index"
```

---

### Task 4: Pixi scene builder

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/render/webgl/scene.ts`

No automated test — pure side-effect on Pixi `Container`s. Verified manually in Task 11.

- [ ] **Step 1: Write `scene.ts`**

Create `src/render/webgl/scene.ts`:

```typescript
import * as PIXI from 'pixi.js'
import type { Scene } from '../layout'
import {
  CONTAINER_FILL,
  LANE_FILL,
  MODULE_FILL,
  MODULE_STROKE,
  severityFill,
  Severity,
  layerHueToHex,
} from './styles'

export interface PixiScene {
  root: PIXI.Container
  lanesLayer: PIXI.Container
  containersLayer: PIXI.Container
  modulesLayer: PIXI.Container
  edgesLayer: PIXI.Container
  destroy(): void
}

export const buildPixiScene = (scene: Scene): PixiScene => {
  const root = new PIXI.Container()
  const lanesLayer = new PIXI.Container()
  const containersLayer = new PIXI.Container()
  const modulesLayer = new PIXI.Container()
  const edgesLayer = new PIXI.Container()
  root.addChild(lanesLayer, containersLayer, modulesLayer, edgesLayer)

  // Lanes
  for (const l of scene.lanes) {
    const g = new PIXI.Graphics()
    g.rect(l.x, l.y, l.width, l.height)
    g.fill({ color: LANE_FILL, alpha: 0.4 })
    g.stroke({ color: layerHueToHex(l.hue, 70, 0.04), width: 1, alpha: 0.6 })
    lanesLayer.addChild(g)
  }

  // Containers
  for (const c of scene.containers) {
    const g = new PIXI.Graphics()
    g.roundRect(c.absX, c.absY, c.width, c.height, 6)
    g.fill({ color: CONTAINER_FILL, alpha: 0.65 })
    g.stroke({ color: layerHueToHex(parseHue(c.layer), 60, 0.08), width: 1.2, alpha: 0.8 })
    containersLayer.addChild(g)
  }

  // Modules
  for (const [id, p] of Object.entries(scene.modulePos)) {
    const g = new PIXI.Graphics()
    const sev = (p.mod.severity ?? null) as Severity | null
    const fill = sev ? severityFill(sev) : MODULE_FILL
    const alpha = sev ? 0.18 : 0.55
    g.roundRect(p.x, p.y, p.w, p.h, p.leaf ? 2 : 4)
    g.fill({ color: fill, alpha })
    g.stroke({
      color: sev ? severityFill(sev) : MODULE_STROKE,
      width: sev ? 1.6 : 1,
      alpha: sev ? 0.9 : 0.5,
    })
    g.label = id
    modulesLayer.addChild(g)
  }

  // Edges
  for (const e of scene.edges) {
    const g = new PIXI.Graphics()
    const color = e.violation ? severityFill('error') : MODULE_STROKE
    const alpha = e.violation ? 0.85 : 0.25
    g.moveTo(e.x1, e.y1)
    g.lineTo(e.x2, e.y2)
    g.stroke({ color, width: e.violation ? 1.6 : 0.8, alpha })
    edgesLayer.addChild(g)
  }

  return {
    root,
    lanesLayer,
    containersLayer,
    modulesLayer,
    edgesLayer,
    destroy() {
      root.destroy({ children: true })
    },
  }
}

// Layer ids look like "app" / "shared" / "core". Convert to a deterministic
// hue 0..360 — same algorithm as src/render/adapt.ts uses.
const parseHue = (layer: string): number => {
  let h = 0
  for (let i = 0; i < layer.length; i++) h = (h * 31 + layer.charCodeAt(i)) >>> 0
  return h % 360
}
```

- [ ] **Step 2: Typecheck**

```bash
npm run typecheck
```

Expected: no errors. If Pixi v8 API mismatch (e.g. `Graphics.rect` not a function) — check `node_modules/pixi.js/lib/scene/graphics/shared/Graphics.d.ts` and adjust to the actual installed v8 API.

- [ ] **Step 3: Commit**

```bash
git add src/render/webgl/scene.ts
git commit -m "feat(viz/webgl): Pixi scene builder from layout Scene"
```

---

### Task 5: `<GraphWebGL/>` component skeleton (Pixi lifecycle)

**Files:**
- Create: `frontend/stratum-visualizer-frontend/src/components/GraphWebGL.tsx`

- [ ] **Step 1: Write the skeleton component**

Create `src/components/GraphWebGL.tsx`:

```typescript
import { Component, createEffect, onCleanup, onMount } from 'solid-js'
import * as PIXI from 'pixi.js'
import { Viewport } from 'pixi-viewport'
import type { Scene } from '../render/layout'
import { DesignData } from '../render/design'
import {
  Filters,
  HoveredEdge,
  HoveredMod,
  Tweaks,
  Viewport as ViewportState,
} from '../state'
import { buildPixiScene, PixiScene } from '../render/webgl/scene'

interface GraphWebGLProps {
  scene: Scene
  data: DesignData
  filters: Filters
  tweaks: Tweaks
  selectedId: string | null
  hoveredId: string | null
  focusedCycle: string | null
  onSelect: (id: string) => void
  onHover: (h: HoveredMod | null) => void
  onHoverEdge: (h: HoveredEdge | null) => void
  hoveredEdge: HoveredEdge | null
  viewport: ViewportState
  onViewportChange: (v: ViewportState) => void
}

export const GraphWebGL: Component<GraphWebGLProps> = (props) => {
  let hostRef!: HTMLDivElement
  let app: PIXI.Application | null = null
  let pxViewport: Viewport | null = null
  let pixiScene: PixiScene | null = null

  onMount(() => {
    void initPixi()
  })

  const initPixi = async () => {
    app = new PIXI.Application()
    await app.init({
      resizeTo: hostRef,
      backgroundAlpha: 0,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    })
    hostRef.appendChild(app.canvas)

    pxViewport = new Viewport({
      screenWidth: hostRef.clientWidth,
      screenHeight: hostRef.clientHeight,
      worldWidth: props.scene.width,
      worldHeight: props.scene.height,
      events: app.renderer.events,
    })
    pxViewport
      .drag()
      .pinch()
      .wheel({ smooth: 8 })
      .decelerate({ friction: 0.92 })
      .clampZoom({ minScale: 0.05, maxScale: 4 })

    app.stage.addChild(pxViewport)

    pxViewport.on('moved', () => {
      if (!pxViewport) return
      props.onViewportChange({
        x: pxViewport.x,
        y: pxViewport.y,
        zoom: pxViewport.scale.x,
      })
    })

    rebuildScene()
  }

  const rebuildScene = () => {
    if (!pxViewport) return
    if (pixiScene) {
      pxViewport.removeChild(pixiScene.root)
      pixiScene.destroy()
    }
    pixiScene = buildPixiScene(props.scene)
    pxViewport.addChild(pixiScene.root)
    pxViewport.worldWidth = props.scene.width
    pxViewport.worldHeight = props.scene.height
  }

  // Rebuild whenever Scene reference changes (layout produced a new object).
  createEffect(() => {
    void props.scene
    if (pxViewport && app) rebuildScene()
  })

  // External viewport changes (e.g. fit-to-bounds from App.tsx).
  createEffect(() => {
    const v = props.viewport
    if (!pxViewport) return
    if (pxViewport.x === v.x && pxViewport.y === v.y && pxViewport.scale.x === v.zoom) return
    pxViewport.scale.set(v.zoom)
    pxViewport.position.set(v.x, v.y)
  })

  onCleanup(() => {
    if (pixiScene) pixiScene.destroy()
    if (pxViewport) pxViewport.destroy({ children: true })
    if (app) app.destroy(true, { children: true, texture: true })
    pixiScene = null
    pxViewport = null
    app = null
  })

  return <div class="strat-graph-webgl" ref={hostRef} />
}
```

- [ ] **Step 2: Typecheck**

```bash
npm run typecheck
```

Expected: no errors. If Pixi `events` accessor differs in installed v8 minor — check the actual API; in 8.0–8.5 it's `app.renderer.events`.

- [ ] **Step 3: Commit**

```bash
git add src/components/GraphWebGL.tsx
git commit -m "feat(viz/webgl): GraphWebGL component — Pixi lifecycle + viewport"
```

---

### Task 6: HTML label overlay synced to viewport

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/src/components/GraphWebGL.tsx`

- [ ] **Step 1: Add overlay rendering**

In `GraphWebGL.tsx`, change the JSX return to include a label overlay div, and add a transform-sync ticker. Replace the bottom of the component:

```typescript
  // ── Label overlay sync ────────────────────────────────────────────────
  let overlayRef!: HTMLDivElement

  const syncOverlay = () => {
    if (!pxViewport || !overlayRef) return
    const x = pxViewport.x
    const y = pxViewport.y
    const s = pxViewport.scale.x
    overlayRef.style.transform = `translate(${x}px, ${y}px) scale(${s})`
  }

  // Subscribe to every Pixi ticker frame; sync is one writeAttribute, cheap.
  onMount(() => {
    const tick = () => syncOverlay()
    const interval = window.setInterval(() => {
      if (app) {
        app.ticker.add(tick)
        window.clearInterval(interval)
      }
    }, 16)
    onCleanup(() => {
      window.clearInterval(interval)
      if (app) app.ticker.remove(tick)
    })
  })

  return (
    <div class="strat-graph-webgl" ref={hostRef}>
      <div class="webgl-labels-overlay" ref={overlayRef}>
        {Object.entries(props.scene.modulePos).map(([id, p]) => (
          <div
            class="webgl-module-label"
            data-id={id}
            style={{
              transform: `translate(${p.x + 4}px, ${p.y + 2}px)`,
              'max-width': `${p.w - 8}px`,
            }}
          >
            {p.mod.label}
          </div>
        ))}
      </div>
    </div>
  )
}
```

Note: `Object.entries(props.scene.modulePos)` runs every time Solid evaluates the return — Solid memoises by reference, so when `props.scene` doesn't change, labels DOM stays. When `props.scene` changes (rebuildScene), Solid re-creates the overlay DOM.

- [ ] **Step 2: Typecheck**

```bash
npm run typecheck
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/GraphWebGL.tsx
git commit -m "feat(viz/webgl): HTML overlay for module labels, synced to viewport"
```

---

### Task 7: Pointer events → hit-test → callbacks

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/src/components/GraphWebGL.tsx`

- [ ] **Step 1: Add hit-test wiring**

In `GraphWebGL.tsx`:

1. Add `import { buildHitTest, HitTestIndex } from '../render/webgl/hitTest'` at the top.
2. Add a memoised hit-test that rebuilds with the scene:

```typescript
  let hitIndex: HitTestIndex | null = null
  let lastHoverId: string | null = null
  let pointerThrottle = 0

  const updateHitIndex = () => {
    hitIndex = buildHitTest(props.scene)
  }

  createEffect(() => {
    void props.scene
    updateHitIndex()
  })

  const onPointerMove = (e: PointerEvent) => {
    if (!pxViewport || !hitIndex) return
    const now = performance.now()
    if (now - pointerThrottle < 16) return
    pointerThrottle = now
    const rect = hostRef.getBoundingClientRect()
    const localX = (e.clientX - rect.left - pxViewport.x) / pxViewport.scale.x
    const localY = (e.clientY - rect.top - pxViewport.y) / pxViewport.scale.x
    const id = hitIndex.queryPoint(localX, localY)
    if (id !== lastHoverId) {
      lastHoverId = id
      props.onHover(id ? { id, x: e.clientX, y: e.clientY } : null)
    }
  }

  const onPointerClick = (e: PointerEvent) => {
    if (!pxViewport || !hitIndex) return
    const rect = hostRef.getBoundingClientRect()
    const localX = (e.clientX - rect.left - pxViewport.x) / pxViewport.scale.x
    const localY = (e.clientY - rect.top - pxViewport.y) / pxViewport.scale.x
    const id = hitIndex.queryPoint(localX, localY)
    if (id) props.onSelect(id)
  }

  onMount(() => {
    hostRef.addEventListener('pointermove', onPointerMove)
    hostRef.addEventListener('click', onPointerClick)
    onCleanup(() => {
      hostRef.removeEventListener('pointermove', onPointerMove)
      hostRef.removeEventListener('click', onPointerClick)
    })
  })
```

Insert these blocks inside the component, after the `rebuildScene` definition and before the `return`. Note: this prototype does **not** wire `onHoverEdge` — edges in WebGL would need a separate edge hit-test (line distance). Edges-only hovering is out of scope per design §8.

- [ ] **Step 2: Typecheck**

```bash
npm run typecheck
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/GraphWebGL.tsx
git commit -m "feat(viz/webgl): pointer hit-test → onHover/onSelect"
```

---

### Task 8: CSS additions for canvas + label overlay

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/src/styles.css`

- [ ] **Step 1: Append WebGL-specific styles**

Append these rules to the **end** of `src/styles.css`:

```css
/* ── WebGL renderer prototype ─────────────────────────────────────────── */

.strat-graph-webgl {
  position: absolute;
  inset: 0;
  overflow: hidden;
  background: var(--bg-1);
  cursor: grab;
}

.strat-graph-webgl:active {
  cursor: grabbing;
}

.strat-graph-webgl canvas {
  display: block;
  width: 100%;
  height: 100%;
}

.webgl-labels-overlay {
  position: absolute;
  inset: 0;
  pointer-events: none;
  transform-origin: 0 0;
  /* will-change keeps the overlay on its own layer — sync writes don't
     repaint the canvas. */
  will-change: transform;
}

.webgl-module-label {
  position: absolute;
  top: 0;
  left: 0;
  transform-origin: 0 0;
  font-family: var(--font-sans);
  font-size: 10px;
  color: var(--tx-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
  user-select: none;
}
```

- [ ] **Step 2: Build to confirm CSS loads**

```bash
npm run build
```

Expected: build succeeds, `dist/assets/index-*.css` exists.

- [ ] **Step 3: Commit**

```bash
git add src/styles.css
git commit -m "style(viz/webgl): canvas + label overlay rules"
```

---

### Task 9: Feature flag — `renderer` signal + URL + localStorage

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/src/App.tsx`

- [ ] **Step 1: Add the renderer signal**

In `App.tsx`, add after the other `createSignal` declarations near line 55:

```typescript
  const [renderer, setRenderer] = createSignal<'svg' | 'webgl'>(readInitialRenderer())
  createEffect(() => {
    const r = renderer()
    const url = new URL(window.location.href)
    url.searchParams.set('renderer', r)
    window.history.replaceState({}, '', url.toString())
    try {
      localStorage.setItem('stratum.renderer', r)
    } catch (_e) {
      // localStorage may be blocked (private mode) — silently degrade.
    }
  })
```

Add this helper outside the component (before `const App: Component = …`):

```typescript
const readInitialRenderer = (): 'svg' | 'webgl' => {
  const fromUrl = new URLSearchParams(window.location.search).get('renderer')
  if (fromUrl === 'webgl' || fromUrl === 'svg') return fromUrl
  try {
    const v = localStorage.getItem('stratum.renderer')
    if (v === 'webgl' || v === 'svg') return v
  } catch (_e) {
    /* ignore */
  }
  return 'svg'
}
```

- [ ] **Step 2: Add `GraphWebGL` import**

Near the top of `App.tsx` add:

```typescript
import { GraphWebGL } from './components/GraphWebGL'
```

- [ ] **Step 3: Swap `<Graph>` with `<Show>` over renderer**

Find the existing `<Graph ... />` element (around line 226) and wrap it. Replace:

```tsx
                      <Graph
                        scene={sc()}
                        ...
                        onViewportChange={setViewport}
                      />
```

with:

```tsx
                      <Show
                        when={renderer() === 'webgl'}
                        fallback={
                          <Graph
                            scene={sc()}
                            data={d()}
                            filters={filters()}
                            tweaks={tweaks()}
                            selectedId={selected()}
                            hoveredId={hovered()?.id ?? null}
                            focusedCycle={focusedCycle()}
                            onSelect={handleSelect}
                            onHover={setHovered}
                            onHoverEdge={setHoveredEdge}
                            hoveredEdge={hoveredEdge()}
                            viewport={viewport()}
                            onViewportChange={setViewport}
                          />
                        }
                      >
                        <GraphWebGL
                          scene={sc()}
                          data={d()}
                          filters={filters()}
                          tweaks={tweaks()}
                          selectedId={selected()}
                          hoveredId={hovered()?.id ?? null}
                          focusedCycle={focusedCycle()}
                          onSelect={handleSelect}
                          onHover={setHovered}
                          onHoverEdge={setHoveredEdge}
                          hoveredEdge={hoveredEdge()}
                          viewport={viewport()}
                          onViewportChange={setViewport}
                        />
                      </Show>
```

- [ ] **Step 4: Pass renderer toggle to `<Header>`**

In `App.tsx`, add to the `<Header>` invocation:

```tsx
            <Header
              data={d()}
              filters={filters()}
              setFilters={setFilters}
              paletteOpen={paletteOpen()}
              setPaletteOpen={setPaletteOpen}
              onSelect={handleSelect}
              onResetView={fitToBounds}
              onTweaksToggle={() =>
                setTweaks({ ...tweaks(), haloThick: !tweaks().haloThick })
              }
              renderer={renderer()}
              onRendererToggle={() => setRenderer(renderer() === 'svg' ? 'webgl' : 'svg')}
            />
```

(Header.tsx will receive these props in Task 10.)

- [ ] **Step 5: Typecheck**

```bash
npm run typecheck
```

Expected: error on `<Header>` because the two new props don't yet exist in `HeaderProps`. That's fine — Task 10 adds them. Skip commit until Task 10.

---

### Task 10: Renderer toggle in `<Header>`

**Files:**
- Modify: `frontend/stratum-visualizer-frontend/src/components/Header.tsx`

- [ ] **Step 1: Extend `HeaderProps`**

In `Header.tsx`, modify the `HeaderProps` interface (around line 12). Add two new fields:

```typescript
interface HeaderProps {
  data: DesignData
  filters: Filters
  setFilters: (f: Filters) => void
  paletteOpen: boolean
  setPaletteOpen: (v: boolean) => void
  onSelect: (id: string) => void
  onResetView: () => void
  onTweaksToggle: () => void
  renderer: 'svg' | 'webgl'
  onRendererToggle: () => void
}
```

- [ ] **Step 2: Add toggle UI**

Inside the `<div class="hd-actions">` block (around line 69), add the renderer toggle button BEFORE the existing fit/⚙ buttons:

```tsx
        <div class="hd-actions">
          <button
            class="hd-btn hd-btn-renderer"
            onClick={props.onRendererToggle}
            title={`Renderer: ${props.renderer.toUpperCase()} (click to switch)`}
          >
            {props.renderer === 'webgl' ? '🌐 WebGL' : '◇ SVG'}
          </button>
          <button class="hd-btn" onClick={props.onResetView} title="Fit to bounds (Esc)">
            ⤢ fit
          </button>
          <button class="hd-btn" onClick={props.onTweaksToggle} title="Open tweaks">
            ⚙
          </button>
        </div>
```

- [ ] **Step 3: Typecheck + build**

```bash
npm run typecheck
npm run build
```

Expected: both succeed. The dist build now contains both renderers.

- [ ] **Step 4: Commit all renderer-switching changes together**

```bash
git add src/App.tsx src/components/Header.tsx
git commit -m "feat(viz): feature-flag SVG | WebGL renderer + header toggle"
```

---

### Task 11: Manual smoke on `tiny-ts`

**Files:** none modified. This is verification, not implementation.

- [ ] **Step 1: Build the frontend**

```bash
cd frontend/stratum-visualizer-frontend
npm run build
```

Expected: build succeeds, no warnings about missing exports.

- [ ] **Step 2: Build stratum-lint with embedded dist**

```bash
cd ../..
cargo build --release -p stratum-lint
```

Expected: build succeeds. `target/release/stratum-lint(.exe)` exists.

- [ ] **Step 3: Run visualizer on tiny-ts fixture**

```bash
./target/release/stratum-lint visualize ./tests/fixtures/tiny-ts --port 18080
```

Expected: server starts, prints "Listening on http://127.0.0.1:18080".

- [ ] **Step 4: Open in browser and verify SVG renderer (default)**

Open `http://localhost:18080`. Verify:
- Page loads, scene visible.
- Header shows `◇ SVG` button on the right.
- Click a module → DetailsPanel shows.

- [ ] **Step 5: Toggle to WebGL**

Click `◇ SVG` button. Expected:
- Button now reads `🌐 WebGL`.
- Canvas replaces SVG; same scene geometry, recognisable.
- Module labels visible as HTML overlay.
- Hover over a module → tooltip appears.
- Click a module → DetailsPanel shows.
- Pan (drag) and zoom (wheel) work.
- URL now contains `?renderer=webgl`.

- [ ] **Step 6: Refresh page**

Reload. WebGL persists (localStorage). Toggle back to SVG. Confirm both directions work.

- [ ] **Step 7: If anything is broken, stop and triage**

Common issues:
- Pixi v8 API mismatch on `Graphics.rect/.roundRect/.fill/.stroke` — open `node_modules/pixi.js/lib/scene/graphics/shared/Graphics.d.ts` and rewrite calls.
- `pixi-viewport` peer dep mismatch — see Appendix B.
- Overlay labels offset wrong — sync ticker not registered; verify the `app.ticker.add(tick)` interval fires (add a `console.log` if needed).

Do not commit triage findings as separate fixes — fold them into the touching task's commit by amending the relevant commit (`git commit --amend`) on the same branch.

---

### Task 12: A/B trace on web-client + report

**Files:**
- Create: `docs/integration/2026-05-23-webgl-prototype-trace.md`

- [ ] **Step 1: Run visualizer on web-client**

```bash
./target/release/stratum-lint visualize D:/web-projects/web-client --port 18080
```

Expected: server starts.

- [ ] **Step 2: SVG trace**

In Chrome:
1. Open `http://localhost:18080?renderer=svg`
2. Wait for fit-to-bounds (scene visible).
3. DevTools → Performance tab → Record.
4. For 5 seconds: scroll-wheel zoom in/out + drag-pan around.
5. Stop recording. Note: frame-time p50, p95 (visible in the FPS chart breakdown), total scripting time, layout time, paint time.
6. Save export to `target/traces/webgl-prototype-svg.json` (DevTools menu → Save profile).

- [ ] **Step 3: WebGL trace**

1. Click the `◇ SVG` toggle → switches to `🌐 WebGL`.
2. Repeat the 5-second wheel+drag scenario.
3. Save export to `target/traces/webgl-prototype-webgl.json`.

- [ ] **Step 4: Write the report**

Create `docs/integration/2026-05-23-webgl-prototype-trace.md`:

```markdown
# WebGL prototype — web-client A/B trace

**Date:** 2026-05-23
**Target:** D:\web-projects\web-client (~3000 modules)
**Stratum:** `<git rev-parse --short HEAD>`
**Browser:** Chrome <version>
**Machine:** <OS, CPU, RAM>

## Scenario

5-second continuous wheel-zoom + drag-pan, same starting viewport, same browser session, toggled mid-trace.

## Results

| Metric | SVG | WebGL | WebGL vs SVG |
|---|---|---|---|
| Frame time p50 (ms) | … | … | … |
| Frame time p95 (ms) | … | … | … |
| First paint (ms, layout-end → first frame) | … | … | … |
| JS heap (MB, after full render) | … | … | … |
| Cold render (ms, init → interactive) | … | … | … |

## Decision

Per design `docs/superpowers/specs/2026-05-23-webgl-renderer-prototype-design.md` §7:

- [ ] FAIL — `WebGL_p95 ≥ SVG_p95`
- [ ] AMBIGUOUS — `SVG_p95 × 0.7 ≤ WebGL_p95 < SVG_p95`
- [ ] PASS — `WebGL_p95 < SVG_p95 × 0.7`

(Tick exactly one.)

## Notes / artefacts

- Trace files: `target/traces/webgl-prototype-{svg,webgl}.json`
- Observations: <e.g. "WebGL still rebuilds full scene on every filter toggle; could batch">
```

Fill in the actual numbers from the traces.

- [ ] **Step 5: Commit the report**

```bash
git add docs/integration/2026-05-23-webgl-prototype-trace.md
git commit -m "docs(integration): WebGL prototype A/B trace on web-client"
```

- [ ] **Step 6: Hand decision back to user**

Surface the report. The decision branch (FAIL / AMBIGUOUS / PASS) drives the next action — out of scope for this plan.

---

## Appendix A — Roll-back

If the trace says FAIL, here's the cleanup:

```bash
git rm frontend/stratum-visualizer-frontend/src/components/GraphWebGL.tsx
git rm frontend/stratum-visualizer-frontend/src/render/webgl/scene.ts
git rm frontend/stratum-visualizer-frontend/src/render/webgl/styles.ts
git rm frontend/stratum-visualizer-frontend/src/render/webgl/styles.test.ts
git rm frontend/stratum-visualizer-frontend/src/render/webgl/hitTest.ts
git rm frontend/stratum-visualizer-frontend/src/render/webgl/hitTest.test.ts
```

Revert the patches to `App.tsx`, `Header.tsx`, `styles.css`, `package.json` (remove `pixi.js`, `pixi-viewport`). `npm install` to refresh lockfile. Commit as `revert: drop WebGL renderer prototype after failed A/B`.

## Appendix B — Manual pan/zoom fallback

If `pixi-viewport` peer-dep is incompatible with the installed Pixi v8, replace `Viewport` usage in `GraphWebGL.tsx` with a plain `PIXI.Container` and wire pan/zoom via DOM events:

```typescript
const camera = new PIXI.Container()
app.stage.addChild(camera)
let dragging = false
let lastX = 0, lastY = 0

hostRef.addEventListener('pointerdown', (e) => {
  dragging = true
  lastX = e.clientX
  lastY = e.clientY
})
hostRef.addEventListener('pointerup', () => { dragging = false })
hostRef.addEventListener('pointermove', (e) => {
  if (!dragging) return
  camera.x += e.clientX - lastX
  camera.y += e.clientY - lastY
  lastX = e.clientX
  lastY = e.clientY
})
hostRef.addEventListener('wheel', (e) => {
  e.preventDefault()
  const factor = e.deltaY < 0 ? 1.1 : 0.9
  const rect = hostRef.getBoundingClientRect()
  const px = e.clientX - rect.left
  const py = e.clientY - rect.top
  const dx = (px - camera.x) * (1 - factor)
  const dy = (py - camera.y) * (1 - factor)
  camera.scale.x *= factor
  camera.scale.y *= factor
  camera.x += dx
  camera.y += dy
}, { passive: false })
```

The rest of the `GraphWebGL` component stays — `camera.addChild(pixiScene.root)` instead of `pxViewport.addChild(...)`.
