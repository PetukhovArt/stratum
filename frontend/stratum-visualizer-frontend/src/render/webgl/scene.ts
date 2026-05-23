import * as PIXI from 'pixi.js'
import type { LaidEdge, LaneInfo, ModulePos, Scene } from '../layout'
import type { Filters, Tweaks } from '../../state'
import {
  HOVER_OUTLINE,
  MODULE_STROKE,
  SELECT_OUTLINE,
  STAGE_HUE,
  SHARED_HUE,
  oklchToRgbHex,
  severityFill,
  Severity,
} from './styles'

export interface PixiScene {
  root: PIXI.Container
  /** Update hover highlight without rebuilding the whole scene. */
  setHover(id: string | null): void
  /** Update selection highlight without rebuilding the whole scene. */
  setSelected(id: string | null): void
  /**
   * Update label visibility based on viewport zoom. Each label was
   * precomputed with its `minZoom` (when its on-screen width hits the
   * readability threshold); this just flips `.visible` per-label.
   */
  setZoom(zoom: number): void
  destroy(): void
}

export interface BuildOptions {
  filters: Filters
  tweaks: Tweaks
  highlightId?: string | null
  focusedCycle?: string | null
}

// ─── helpers ────────────────────────────────────────────────────────────────

const buildContainerLayerMap = (scene: Scene): Map<string, string> => {
  const m = new Map<string, string>()
  for (const c of scene.containers) m.set(c.id, c.layer)
  return m
}

const buildLaneByLayerId = (scene: Scene): Map<string, LaneInfo> => {
  const m = new Map<string, LaneInfo>()
  for (const l of scene.lanes) m.set(l.id, l)
  return m
}

const buildVisibleMods = (
  scene: Scene,
  filters: Filters,
  containerLayer: Map<string, string>,
): Set<string> => {
  const visible = new Set<string>()
  for (const [id, p] of Object.entries(scene.modulePos)) {
    const lay = containerLayer.get(p.mod.container) ?? ''
    if (filters.disabledLayers.has(lay)) continue
    if (filters.disabledContainers.has(p.mod.container)) continue
    if (
      filters.onlyViolators &&
      p.mod.violations.length === 0 &&
      p.mod.descError === 0 &&
      p.mod.descWarn === 0
    ) {
      continue
    }
    if (filters.stageFilter !== null && p.mod.stage !== filters.stageFilter) continue
    visible.add(id)
  }
  return visible
}

const isEdgeVisible = (
  e: LaidEdge,
  filters: Filters,
  visibleMods: Set<string>,
  cycleEdgeIds: Set<number> | null,
  highlightId: string | null,
): boolean => {
  if (cycleEdgeIds) return cycleEdgeIds.has(e.i)
  if (filters.onlyViolators && !e.violation) return false
  if (!visibleMods.has(e.source) || !visibleMods.has(e.target)) return false
  if (!filters.edgeKinds.has(e.kind)) return false
  if (filters.connectionMode === 'minimal' && !e.violation) {
    if (!highlightId || (e.source !== highlightId && e.target !== highlightId)) return false
  }
  return true
}

const hueFor = (
  pos: ModulePos,
  tweaks: Tweaks,
  laneByLayerId: Map<string, LaneInfo>,
  containerLayer: Map<string, string>,
): number => {
  if (tweaks.colorMode === 'stage') {
    return STAGE_HUE[Math.min(Math.max(pos.mod.stage, 1), 4) - 1] ?? 220
  }
  const layerId = containerLayer.get(pos.mod.container)
  if (!layerId) return 240
  return laneByLayerId.get(layerId)?.hue ?? 240
}

const moduleFill = (pos: ModulePos, hue: number): number => {
  const sev = pos.mod.severity as Severity | null
  if (sev) return severityFill(sev)
  if (pos.mod.kind === 'shared') return oklchToRgbHex(38, 0.07, SHARED_HUE)
  // Compound: darker layer-tinted to read as a card. Leaf: lighter.
  return pos.leaf ? oklchToRgbHex(38, 0.1, hue) : oklchToRgbHex(26, 0.08, hue)
}

const moduleFillAlpha = (pos: ModulePos): number => {
  if (pos.mod.severity) return 0.18
  return pos.leaf ? 0.65 : 0.55
}

const moduleStroke = (pos: ModulePos, hue: number, isComposer: boolean): number => {
  const sev = pos.mod.severity as Severity | null
  if (sev) return severityFill(sev)
  if (isComposer) return oklchToRgbHex(78, 0.16, hue)
  if (pos.mod.kind === 'shared') return oklchToRgbHex(72, 0.1, SHARED_HUE)
  return oklchToRgbHex(72, 0.1, hue)
}

const rounding = (pos: ModulePos): number => {
  if (pos.depth === 0) return 6
  if (pos.leaf) return 2
  return 4
}

// ─── main builder ───────────────────────────────────────────────────────────

export const buildPixiScene = (scene: Scene, opts: BuildOptions): PixiScene => {
  const { filters, tweaks, highlightId = null, focusedCycle = null } = opts
  const containerLayer = buildContainerLayerMap(scene)
  const laneByLayerId = buildLaneByLayerId(scene)
  const visibleMods = buildVisibleMods(scene, filters, containerLayer)

  let cycleEdgeIds: Set<number> | null = null
  let cycleModIds: Set<string> | null = null
  if (focusedCycle) {
    cycleEdgeIds = new Set()
    cycleModIds = new Set()
    for (const e of scene.edges) {
      if (e.violation === focusedCycle) {
        cycleEdgeIds.add(e.i)
        cycleModIds.add(e.source)
        cycleModIds.add(e.target)
      }
    }
  }

  // Cached lookup of modulePos by id — Object.entries is used in inner loops
  // for hover/select overlays.
  const posById: Record<string, ModulePos> = scene.modulePos

  const root = new PIXI.Container()
  root.eventMode = 'none'
  root.interactiveChildren = false

  // Layer order: lanes < containers < severityHalo < modules (compounds) <
  // modules (leaves) < segmentMarkers < composerMarkers < sharedDots < edges <
  // hoverOverlay < selectOverlay. Keeping leaves above compounds so a small
  // leaf inside a compound card always paints on top.

  const lanesG = new PIXI.Graphics()
  const containersG = new PIXI.Graphics()
  const haloG = new PIXI.Graphics()
  const compoundsG = new PIXI.Graphics()
  const leavesG = new PIXI.Graphics()
  const composerG = new PIXI.Graphics()
  const sharedDotG = new PIXI.Graphics()
  const edgesG = new PIXI.Graphics()
  const labelsLayer = new PIXI.Container()
  const hoverG = new PIXI.Graphics()
  const selectG = new PIXI.Graphics()
  labelsLayer.eventMode = 'none'
  labelsLayer.interactiveChildren = false

  root.addChild(
    lanesG,
    containersG,
    haloG,
    compoundsG,
    leavesG,
    composerG,
    sharedDotG,
    edgesG,
    labelsLayer,
    hoverG,
    selectG,
  )

  // ─── Lanes ────────────────────────────────────────────────────────────────
  // Three layerMode behaviours:
  //   - 'swimlanes': full tinted background across scene width
  //   - 'borders': very subtle tone, just enough to telegraph the layer line
  //   - 'none': skip entirely
  if (tweaks.layerMode !== 'none') {
    for (const l of scene.lanes) {
      const dimmed = filters.disabledLayers.has(l.id)
      if (tweaks.layerMode === 'swimlanes') {
        lanesG
          .rect(0, l.y, scene.width, l.height)
          .fill({ color: oklchToRgbHex(60, 0.18, l.hue), alpha: dimmed ? 0.04 : 0.045 })
          .stroke({ color: oklchToRgbHex(60, 0.04, 0), width: 1, alpha: 0.04 })
        // 4-px accent stripe at the left edge.
        lanesG
          .rect(0, l.y, 4, l.height)
          .fill({ color: oklchToRgbHex(70, 0.2, l.hue), alpha: dimmed ? 0.2 : 0.7 })
      } else {
        // 'borders' — paint a single 2-px line at the lane top so the visual
        // hierarchy still telegraphs which layer you're in.
        lanesG
          .rect(0, l.y, scene.width, 2)
          .fill({ color: oklchToRgbHex(70, 0.16, l.hue), alpha: dimmed ? 0.2 : 0.55 })
      }
    }
  }

  // ─── Containers ───────────────────────────────────────────────────────────
  for (const c of scene.containers) {
    if (filters.disabledLayers.has(c.layer)) continue
    if (filters.disabledContainers.has(c.id)) continue
    const lane = laneByLayerId.get(c.layer)
    const hue = lane?.hue ?? 240
    const fillColor = oklchToRgbHex(65, 0.1, hue)
    const strokeColor =
      tweaks.layerMode === 'borders'
        ? oklchToRgbHex(70, 0.16, hue)
        : oklchToRgbHex(70, 0.02, 0)
    const strokeAlpha = tweaks.layerMode === 'borders' ? 0.75 : 0.08
    const strokeWidth = tweaks.layerMode === 'borders' ? 1.2 : 0.8
    containersG
      .roundRect(c.absX, c.absY, c.width, c.height, 10)
      .fill({ color: fillColor, alpha: 0.05 })
      .stroke({ color: strokeColor, width: strokeWidth, alpha: strokeAlpha })
  }

  // ─── Severity halos (behind modules) ──────────────────────────────────────
  for (const [id, p] of Object.entries(scene.modulePos)) {
    if (!visibleMods.has(id)) continue
    if (cycleModIds && !cycleModIds.has(id)) continue
    const sev = p.mod.severity as Severity | null
    if (!sev) continue
    const r = rounding(p)
    haloG
      .roundRect(p.x - 2, p.y - 2, p.w + 4, p.h + 4, r + 2)
      .fill({ color: severityFill(sev), alpha: 0.22 })
    if (tweaks.haloThick) {
      haloG
        .roundRect(p.x - 4, p.y - 4, p.w + 8, p.h + 8, r + 3)
        .stroke({ color: severityFill(sev), width: 1.4, alpha: 0.3 })
    }
  }

  // ─── Modules (compounds + leaves) ─────────────────────────────────────────
  // Walk in two passes so compounds always paint first (cleaner z-order with
  // batched Graphics — children visually overlay their parent card).
  for (const [id, p] of Object.entries(scene.modulePos)) {
    if (!visibleMods.has(id)) continue
    if (cycleModIds && !cycleModIds.has(id)) continue
    if (p.leaf) continue
    const hue = hueFor(p, tweaks, laneByLayerId, containerLayer)
    const isComposer = p.mod.kind === 'composer'
    compoundsG
      .roundRect(p.x, p.y, p.w, p.h, rounding(p))
      .fill({ color: moduleFill(p, hue), alpha: moduleFillAlpha(p) })
      .stroke({
        color: moduleStroke(p, hue, isComposer),
        width: isComposer ? 1.4 : 0.9,
        alpha: 0.7,
      })
  }
  for (const [id, p] of Object.entries(scene.modulePos)) {
    if (!visibleMods.has(id)) continue
    if (cycleModIds && !cycleModIds.has(id)) continue
    if (!p.leaf) continue
    const hue = hueFor(p, tweaks, laneByLayerId, containerLayer)
    const isComposer = p.mod.kind === 'composer'
    const sev = p.mod.severity as Severity | null
    if (isComposer) {
      // Composer leaves get their stroke drawn dashed in the composer pass
      // below — fill here, skip stroke so we don't double-paint.
      leavesG
        .roundRect(p.x, p.y, p.w, p.h, rounding(p))
        .fill({ color: moduleFill(p, hue), alpha: moduleFillAlpha(p) })
      continue
    }
    leavesG
      .roundRect(p.x, p.y, p.w, p.h, rounding(p))
      .fill({ color: moduleFill(p, hue), alpha: moduleFillAlpha(p) })
      .stroke({
        color: moduleStroke(p, hue, isComposer),
        width: sev ? 1.6 : 1,
        alpha: sev ? 0.95 : 0.7,
      })
  }

  // ─── Composer leaves: dashed perimeter + inner diamond ───────────────────
  // PIXI v8 has no native dashed stroke, so the perimeter is emitted as a
  // sequence of short moveTo+lineTo segments along the 4 straight edges.
  // The leaf rounding (radius 2) makes corner arcs nearly invisible, so we
  // simply leave a tiny gap at each corner instead of stroking arcs.
  const DASH_LEN = 3
  const GAP_LEN = 2
  const addDashedLine = (
    g: PIXI.Graphics,
    x1: number, y1: number, x2: number, y2: number,
  ) => {
    const dx = x2 - x1
    const dy = y2 - y1
    const total = Math.hypot(dx, dy)
    if (total < 0.5) return
    const ux = dx / total
    const uy = dy / total
    let pos = 0
    while (pos < total) {
      const end = Math.min(pos + DASH_LEN, total)
      g.moveTo(x1 + ux * pos, y1 + uy * pos)
       .lineTo(x1 + ux * end, y1 + uy * end)
      pos = end + GAP_LEN
    }
  }
  for (const [id, p] of Object.entries(scene.modulePos)) {
    if (!visibleMods.has(id)) continue
    if (cycleModIds && !cycleModIds.has(id)) continue
    if (p.mod.kind !== 'composer' || !p.leaf) continue
    const hue = hueFor(p, tweaks, laneByLayerId, containerLayer)
    const sev = p.mod.severity as Severity | null
    const r = rounding(p)
    const outlineColor = sev ? severityFill(sev) : oklchToRgbHex(78, 0.16, hue)
    const outlineWidth = sev ? 1.6 : 1.4
    addDashedLine(composerG, p.x + r, p.y,             p.x + p.w - r, p.y)
    addDashedLine(composerG, p.x + p.w, p.y + r,       p.x + p.w,     p.y + p.h - r)
    addDashedLine(composerG, p.x + p.w - r, p.y + p.h, p.x + r,       p.y + p.h)
    addDashedLine(composerG, p.x, p.y + p.h - r,       p.x,           p.y + r)
    composerG.stroke({ color: outlineColor, width: outlineWidth, alpha: sev ? 0.95 : 0.9 })
    // Inner diamond — sized to fit the module, not just a corner marker.
    const cx = p.x + p.w / 2
    const cy = p.y + p.h / 2
    composerG
      .moveTo(cx, p.y + 1)
      .lineTo(p.x + p.w - 1, cy)
      .lineTo(cx, p.y + p.h - 1)
      .lineTo(p.x + 1, cy)
      .closePath()
      .stroke({ color: oklchToRgbHex(84, 0.18, hue), width: 1, alpha: 0.95 })
  }

  // ─── Shared-kind dots (corner mark) ──────────────────────────────────────
  for (const [id, p] of Object.entries(scene.modulePos)) {
    if (!visibleMods.has(id)) continue
    if (cycleModIds && !cycleModIds.has(id)) continue
    if (p.mod.kind !== 'shared' || !p.leaf) continue
    sharedDotG
      .circle(p.x + p.w - 3, p.y + 3, 1.8)
      .fill({ color: oklchToRgbHex(78, 0.1, SHARED_HUE), alpha: 0.9 })
  }

  // ─── Edges ────────────────────────────────────────────────────────────────
  for (const e of scene.edges) {
    if (!isEdgeVisible(e, filters, visibleMods, cycleEdgeIds, highlightId)) continue
    const color = e.violation ? severityFill('error') : MODULE_STROKE
    const alpha = e.violation ? 0.85 : 0.25
    edgesG
      .moveTo(e.x1, e.y1)
      .lineTo(e.x2, e.y2)
      .stroke({ color, width: e.violation ? 1.6 : 0.8, alpha })
  }

  // ─── Labels (BitmapText on canvas) ───────────────────────────────────────
  // Replaces the previous HTML overlay. Reasons:
  //   - HTML labels triggered DOM-mutation storms on every zoom step (set of
  //     visible labels changes constantly via virtualization).
  //   - The overlay wrapper had a CSS transform that browsers re-rendered for
  //     every label child every frame.
  //   - BitmapText reuses a single glyph atlas; thousands of instances batch
  //     into a few GPU draw calls.
  //
  // Visibility is precomputed per label (`minZoom`); a single `setZoom`
  // pass on viewport change flips `.visible` without rebuilding any geometry.
  const LABEL_FONT_SIZE = 11
  const LABEL_COLOR = oklchToRgbHex(88, 0.02, 250)
  const LABEL_CHAR_W = 6.2 // ~px per glyph at fontSize=11 in default sans

  interface LabelEntry {
    node: PIXI.BitmapText
    minZoom: number
    alwaysVisible: boolean
  }
  const labelEntries: LabelEntry[] = []

  const addLabel = (
    text: string,
    x: number,
    y: number,
    worldW: number,
    alwaysVisible: boolean,
  ) => {
    if (!text) return
    // No ellipses — they don't fit in tiny modules anyway and just add noise.
    // For always-visible labels (top compounds, severity rows) we hard-truncate
    // to whatever fits without trailing "…". For ordinary labels we render the
    // FULL text and gate visibility via minZoom: it only appears when the user
    // has zoomed in far enough for the full text to fit cleanly, so a stub
    // like "g" never pollutes the view.
    const availPx = Math.max(0, worldW - 4)
    let shown: string
    let minZoom: number
    if (alwaysVisible) {
      const maxChars = Math.max(1, Math.floor(availPx / LABEL_CHAR_W))
      shown = text.length <= maxChars ? text : text.slice(0, maxChars)
      minZoom = 0
    } else {
      shown = text
      const fullPx = text.length * LABEL_CHAR_W + 4
      minZoom = fullPx / Math.max(worldW, 1)
    }
    if (!shown) return
    const node = new PIXI.BitmapText({
      text: shown,
      style: { fontFamily: 'sans-serif', fontSize: LABEL_FONT_SIZE, fill: LABEL_COLOR },
    })
    node.x = x
    node.y = y
    labelsLayer.addChild(node)
    labelEntries.push({ node, minZoom, alwaysVisible })
  }

  for (const [id, p] of Object.entries(scene.modulePos)) {
    if (!visibleMods.has(id)) continue
    if (cycleModIds && !cycleModIds.has(id)) continue
    const isTopCompound = p.depth === 0 && !p.leaf
    addLabel(
      p.mod.label,
      p.x + 4,
      p.y + 2,
      p.w,
      isTopCompound || !!p.mod.severity,
    )
  }

  // ─── Hover / select overlays — drawn on demand ────────────────────────────
  // Both share the same outline style (different colours). They live as
  // independent Graphics so pointermove redraws only the hover layer, leaving
  // every other layer's cached geometry on the GPU untouched.
  const drawOutline = (g: PIXI.Graphics, pos: ModulePos, color: number, width: number) => {
    const r = rounding(pos)
    g.roundRect(pos.x - 3, pos.y - 3, pos.w + 6, pos.h + 6, r + 3)
      .stroke({ color, width, alpha: 0.95 })
  }

  const setHover = (id: string | null) => {
    hoverG.clear()
    if (!id) return
    const pos = posById[id]
    if (!pos) return
    drawOutline(hoverG, pos, HOVER_OUTLINE, 1.6)
  }

  const setSelected = (id: string | null) => {
    selectG.clear()
    if (!id) return
    const pos = posById[id]
    if (!pos) return
    drawOutline(selectG, pos, SELECT_OUTLINE, 2.2)
  }

  const setZoom = (zoom: number) => {
    for (let i = 0; i < labelEntries.length; i++) {
      const e = labelEntries[i]
      e.node.visible = e.alwaysVisible || zoom >= e.minZoom
    }
  }

  // Apply initial state.
  if (highlightId) setHover(highlightId)
  setZoom(1)

  return {
    root,
    setHover,
    setSelected,
    setZoom,
    destroy() {
      root.destroy({ children: true, context: true })
    },
  }
}

