// Fractal layout — recursive sizing + grid packing.
//
// Every module can have children. Leaves are small squares whose side scales
// gently by sqrt(loc). Compound modules become packed grids of their children.
// Containers are packed wrapping rows inside each lane.

import { DesignData, DesignModule } from './design'

export type Density = 'compact' | 'cozy' | 'roomy'

export interface LayoutOptions {
  density?: Density
  laneMaxWidth?: number
  /** Reserved — vertical/horizontal lane stacking. Currently TB only. */
  direction?: 'TB' | 'LR'
}

export interface LaneInfo {
  id: string
  label: string
  order: number
  hue: number
  modules: number
  errors: number
  warnings: number
  x: number
  y: number
  width: number
  height: number
}

export interface ContainerInfo {
  id: string
  layer: string
  label: string
  errors: number
  warnings: number
  moduleCount: number
  stage: number
  width: number
  height: number
  labelH: number
  absX: number
  absY: number
}

export interface ModulePos {
  x: number
  y: number
  w: number
  h: number
  cx: number
  cy: number
  mod: DesignModule
  leaf: boolean
  depth: number
  childIds: string[]
}

export interface LaidEdge {
  i: number
  source: string
  target: string
  kind: 'static' | 'di' | 'runtime'
  violation: string | null
  x1: number
  y1: number
  x2: number
  y2: number
}

export interface Scene {
  lanes: LaneInfo[]
  containers: ContainerInfo[]
  modulePos: Record<string, ModulePos>
  edges: LaidEdge[]
  width: number
  height: number
}

interface DensityTokens {
  leafSize: number
  leafGap: number
  moduleHeader: number
  modulePadH: number
  modulePadV: number
  modulesPerRow: number
  childrenPerRow: number
  containerPadding: { t: number; r: number; b: number; l: number }
  containerLabelHeight: number
  containerGap: number
  laneGapY: number
  lanePadX: number
  laneLabelWidth: number
}

const DENSITY: Record<Density, DensityTokens> = {
  compact: {
    leafSize: 14,
    leafGap: 3,
    moduleHeader: 15,
    modulePadH: 6,
    modulePadV: 5,
    modulesPerRow: 5,
    childrenPerRow: 4,
    containerPadding: { t: 22, r: 9, b: 9, l: 9 },
    containerLabelHeight: 20,
    containerGap: 14,
    laneGapY: 28,
    lanePadX: 22,
    laneLabelWidth: 100,
  },
  cozy: {
    leafSize: 18,
    leafGap: 4,
    moduleHeader: 18,
    modulePadH: 8,
    modulePadV: 6,
    modulesPerRow: 4,
    childrenPerRow: 3,
    containerPadding: { t: 26, r: 12, b: 12, l: 12 },
    containerLabelHeight: 22,
    containerGap: 20,
    laneGapY: 40,
    lanePadX: 28,
    laneLabelWidth: 100,
  },
  roomy: {
    leafSize: 24,
    leafGap: 6,
    moduleHeader: 22,
    modulePadH: 12,
    modulePadV: 9,
    modulesPerRow: 3,
    childrenPerRow: 3,
    containerPadding: { t: 32, r: 16, b: 16, l: 16 },
    containerLabelHeight: 26,
    containerGap: 28,
    laneGapY: 48,
    lanePadX: 36,
    laneLabelWidth: 100,
  },
}

interface SizedItem {
  id: string
  mod: DesignModule
  w: number
  h: number
  leaf: boolean
  depth: number
  placedKids: PlacedItem[]
}

interface PlacedItem extends SizedItem {
  x: number
  y: number
}

interface Packed {
  width: number
  height: number
  items: PlacedItem[]
}

const sizeModule = (
  modId: string,
  data: DesignData,
  tokens: DensityTokens,
  byId: Map<string, DesignModule>,
  depth: number,
): SizedItem => {
  const mod = byId.get(modId)
  if (!mod) throw new Error(`unknown module ${modId}`)
  const kidIds = data.childIndex[modId] ?? []
  if (kidIds.length === 0) {
    const base = tokens.leafSize
    const scale = Math.min(1.5, 0.7 + 0.5 * Math.sqrt(Math.min(mod.loc, 240) / 240 || 0.4))
    const side = Math.round(base * scale)
    return { id: modId, mod, w: side, h: side, leaf: true, depth, placedKids: [] }
  }
  const sized = kidIds.map((cid) => sizeModule(cid, data, tokens, byId, depth + 1))
  const packed = packGrid(sized, tokens, depth + 1)
  const w = Math.max(packed.width + tokens.modulePadH * 2, 70)
  const h = packed.height + tokens.moduleHeader + tokens.modulePadV
  return { id: modId, mod, w, h, leaf: false, depth, placedKids: packed.items }
}

const packGrid = (items: SizedItem[], tokens: DensityTokens, depth: number): Packed => {
  if (items.length === 0) return { width: 0, height: 0, items: [] }
  // At depth=0 (container top-level), grow column count with N so the container stays
  // roughly 4:1 wide instead of degenerating into a tall narrow column.
  // perRow = max(densityBaseline, ceil(√(N × 4)))  → for N=878 that's ~59 cols.
  const perRow =
    depth === 0
      ? Math.max(tokens.modulesPerRow, Math.ceil(Math.sqrt(items.length * 4)))
      : tokens.childrenPerRow
  const gap = tokens.leafGap
  const placed: PlacedItem[] = []
  let x = 0
  let y = 0
  let rowH = 0
  let col = 0
  let maxRowW = 0

  for (const it of items) {
    const isWide = !it.leaf
    if (isWide && col > 0) {
      maxRowW = Math.max(maxRowW, x)
      x = 0
      y += rowH + gap
      rowH = 0
      col = 0
    }
    if (!isWide && col >= perRow) {
      maxRowW = Math.max(maxRowW, x)
      x = 0
      y += rowH + gap
      rowH = 0
      col = 0
    }
    placed.push({ ...it, x, y })
    x += it.w + gap
    rowH = Math.max(rowH, it.h)
    col += isWide ? perRow : 1
    if (isWide) {
      maxRowW = Math.max(maxRowW, x)
      x = 0
      y += rowH + gap
      rowH = 0
      col = 0
    }
  }
  maxRowW = Math.max(maxRowW, x)
  return { width: Math.max(maxRowW - gap, 0), height: y + rowH, items: placed }
}

interface SizedContainer {
  id: string
  width: number
  height: number
  labelH: number
  packed: Packed
  data: ContainerInfo
}

interface LanePlacement {
  id: string
  x: number
  y: number
  w: number
  h: number
}

const packLane = (
  containers: SizedContainer[],
  maxWidth: number,
  tokens: DensityTokens,
): { width: number; height: number; placements: LanePlacement[] } => {
  const placements: LanePlacement[] = []
  let x = 0
  let y = 0
  let rowH = 0
  let maxRowW = 0
  for (const c of containers) {
    if (x > 0 && x + c.width > maxWidth) {
      maxRowW = Math.max(maxRowW, x)
      x = 0
      y += rowH + tokens.containerGap
      rowH = 0
    }
    placements.push({ id: c.id, x, y, w: c.width, h: c.height })
    x += c.width + tokens.containerGap
    rowH = Math.max(rowH, c.height)
  }
  maxRowW = Math.max(maxRowW, x)
  return { width: maxRowW, height: y + rowH, placements }
}

const emitPositions = (
  item: PlacedItem,
  ox: number,
  oy: number,
  out: Record<string, ModulePos>,
  depth: number,
  tokens: DensityTokens,
): void => {
  const ax = ox + item.x
  const ay = oy + item.y
  const cx = ax + item.w / 2
  const cy = ay + item.h / 2
  out[item.id] = {
    x: ax,
    y: ay,
    w: item.w,
    h: item.h,
    cx,
    cy,
    mod: item.mod,
    leaf: item.leaf,
    depth,
    childIds: item.placedKids.map((k) => k.id),
  }
  if (item.placedKids.length) {
    const innerX = ax + tokens.modulePadH
    const innerY = ay + tokens.moduleHeader
    for (const kid of item.placedKids) emitPositions(kid, innerX, innerY, out, depth + 1, tokens)
  }
}

export const computeLayout = (data: DesignData, options: LayoutOptions = {}): Scene => {
  const tokens = DENSITY[options.density ?? 'cozy']
  // Lanes don't wrap container rows by default — we'd rather grow horizontally
  // than turn the scene into a 1:30 ribbon. Callers can clamp via `laneMaxWidth`
  // if they want a square viewport at the cost of more vertical stacking.
  const laneMaxWidth = options.laneMaxWidth ?? Number.POSITIVE_INFINITY
  const byId = new Map(data.modules.map((m) => [m.id, m]))

  // 1) Size every container.
  const sizedContainers: SizedContainer[] = data.containers.map((c) => {
    const tops = data.topByContainer[c.id] ?? []
    const sized = tops.map((id) => sizeModule(id, data, tokens, byId, 0))
    const packed = packGrid(sized, tokens, 0)
    const labelH = tokens.containerLabelHeight
    const w = Math.max(
      packed.width + tokens.containerPadding.l + tokens.containerPadding.r,
      110,
    )
    const h = packed.height + labelH + tokens.containerPadding.b
    return {
      id: c.id,
      width: w,
      height: h,
      labelH,
      packed,
      data: { ...c, width: w, height: h, labelH, absX: 0, absY: 0 },
    }
  })

  // 2) Pack containers per lane (wrap rows).
  const lanes: LaneInfo[] = data.layers.map((layer) => {
    const inLane = sizedContainers.filter((c) => c.data.layer === layer.id)
    const packed = packLane(inLane, laneMaxWidth - 2 * tokens.lanePadX, tokens)
    return {
      ...layer,
      width: packed.width + 2 * tokens.lanePadX,
      height: packed.height + 2 * tokens.lanePadX,
      x: 0,
      y: 0,
      _placements: packed.placements,
    } as LaneInfo & { _placements: LanePlacement[] }
  })

  // 3) Stack lanes top → bottom.
  const laneOriginX = tokens.laneLabelWidth + 8
  let cursor = 0
  for (const lane of lanes) {
    lane.x = laneOriginX
    lane.y = cursor
    cursor += lane.height + tokens.laneGapY
  }

  // 4) Absolute positions per module.
  const modulePos: Record<string, ModulePos> = {}
  for (const sized of sizedContainers) {
    const lane = lanes.find((l) => l.id === sized.data.layer) as LaneInfo & {
      _placements: LanePlacement[]
    }
    if (!lane) continue
    const placement = lane._placements.find((p) => p.id === sized.id)
    if (!placement) continue
    sized.data.absX = lane.x + tokens.lanePadX + placement.x
    sized.data.absY = lane.y + tokens.lanePadX + placement.y
    const innerX = sized.data.absX + tokens.containerPadding.l
    const innerY = sized.data.absY + sized.labelH
    for (const item of sized.packed.items) {
      emitPositions(item, innerX, innerY, modulePos, 0, tokens)
    }
  }

  // 5) Resolve edges.
  const edges: LaidEdge[] = []
  data.edges.forEach((e, i) => {
    const a = modulePos[e.source]
    const b = modulePos[e.target]
    if (!a || !b) return
    edges.push({
      i,
      source: e.source,
      target: e.target,
      kind: e.kind,
      violation: e.violation,
      x1: a.cx,
      y1: a.cy,
      x2: b.cx,
      y2: b.cy,
    })
  })

  const sceneW =
    lanes.length === 0 ? 0 : Math.max(...lanes.map((l) => l.x + l.width)) + 16
  const lastLane = lanes[lanes.length - 1]
  const sceneH = lastLane ? lastLane.y + lastLane.height + 16 : 0

  // Strip private packing data from the public lane shape.
  const cleanLanes: LaneInfo[] = lanes.map(({ x, y, width, height, ...rest }) => ({
    ...(rest as Omit<LaneInfo, 'x' | 'y' | 'width' | 'height'>),
    x,
    y,
    width,
    height,
  }))

  return {
    lanes: cleanLanes,
    containers: sizedContainers.map((c) => c.data),
    modulePos,
    edges,
    width: sceneW,
    height: sceneH,
  }
}
