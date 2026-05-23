import * as PIXI from 'pixi.js'
import type { LaidEdge, Scene } from '../layout'
import type { Filters } from '../../state'
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

export interface BuildOptions {
  filters: Filters
  highlightId?: string | null
  focusedCycle?: string | null
}

const buildContainerLayerMap = (scene: Scene): Map<string, string> => {
  const m = new Map<string, string>()
  for (const c of scene.containers) m.set(c.id, c.layer)
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

export const buildPixiScene = (scene: Scene, opts: BuildOptions): PixiScene => {
  const { filters, highlightId = null, focusedCycle = null } = opts
  const containerLayer = buildContainerLayerMap(scene)
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

  const root = new PIXI.Container()
  const lanesLayer = new PIXI.Container()
  const containersLayer = new PIXI.Container()
  const modulesLayer = new PIXI.Container()
  const edgesLayer = new PIXI.Container()
  root.addChild(lanesLayer, containersLayer, modulesLayer, edgesLayer)

  // Lanes — always painted (visual backdrop), but dim if layer is disabled.
  for (const l of scene.lanes) {
    const g = new PIXI.Graphics()
    const dimmed = filters.disabledLayers.has(l.id)
    g.rect(l.x, l.y, l.width, l.height)
    g.fill({ color: LANE_FILL, alpha: dimmed ? 0.1 : 0.4 })
    g.stroke({
      color: layerHueToHex(l.hue, 70, 0.04),
      width: 1,
      alpha: dimmed ? 0.15 : 0.6,
    })
    lanesLayer.addChild(g)
  }

  // Containers — skip if disabled or layer disabled.
  for (const c of scene.containers) {
    if (filters.disabledLayers.has(c.layer)) continue
    if (filters.disabledContainers.has(c.id)) continue
    const g = new PIXI.Graphics()
    g.roundRect(c.absX, c.absY, c.width, c.height, 6)
    g.fill({ color: CONTAINER_FILL, alpha: 0.65 })
    g.stroke({
      color: layerHueToHex(parseHue(c.layer), 60, 0.08),
      width: 1.2,
      alpha: 0.8,
    })
    containersLayer.addChild(g)
  }

  // Modules — visibility per filters; cycle-focus tints dim.
  for (const [id, p] of Object.entries(scene.modulePos)) {
    if (!visibleMods.has(id)) continue
    const dimmed = cycleModIds ? !cycleModIds.has(id) : false
    if (dimmed) continue
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

  // Edges — full filter pipeline (kinds, only-violators, minimal/all, cycle-focus).
  for (const e of scene.edges) {
    if (!isEdgeVisible(e, filters, visibleMods, cycleEdgeIds, highlightId)) continue
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
