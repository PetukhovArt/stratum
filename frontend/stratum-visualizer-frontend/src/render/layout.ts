import { DesignContainer, DesignData, DesignLayer, DesignModule, EdgeKindLabel, SeverityLabel } from './design'

export type Density = 'compact' | 'cozy' | 'roomy'

export interface LayoutOptions {
  density?: Density
}

export interface LaneInfo extends DesignLayer {
  x: number
  y: number
  width: number
  height: number
}

export interface ContainerInfo extends DesignContainer {
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
  isLeaf: boolean
  depth: number
  childIds: string[]
}

export interface LaidEdge {
  i: number
  source: string
  target: string
  kind: EdgeKindLabel
  violation: string | null
  severity: SeverityLabel | null
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
  lanePadY: number
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
    lanePadY: 22,
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
    lanePadY: 28,
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
    lanePadY: 36,
  },
}

const LAYOUT = {
  minCompoundWidth: 70,
  minContainerWidth: 110,
  topLevelRowAspect: 4,
  laneLabelWidth: 100,
  laneLabelGap: 8,
  sceneMargin: 16,
} as const

interface SizedItem {
  id: string
  mod: DesignModule
  w: number
  h: number
  isLeaf: boolean
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

interface LayoutContext {
  data: DesignData
  tokens: DensityTokens
}

const sizeModule = (ctx: LayoutContext, modId: string, depth: number): SizedItem => {
  const mod = ctx.data.moduleById.get(modId)
  if (!mod) throw new Error(`unknown module ${modId}`)
  const kidIds = ctx.data.childIndex[modId] ?? []
  if (kidIds.length === 0) {
    const side = ctx.tokens.leafSize
    return { id: modId, mod, w: side, h: side, isLeaf: true, placedKids: [] }
  }
  const sized = kidIds.map((cid) => sizeModule(ctx, cid, depth + 1))
  const packed = packGrid(sized, ctx.tokens, depth + 1)
  const w = Math.max(packed.width + ctx.tokens.modulePadH * 2, LAYOUT.minCompoundWidth)
  const h = packed.height + ctx.tokens.moduleHeader + ctx.tokens.modulePadV
  return { id: modId, mod, w, h, isLeaf: false, placedKids: packed.items }
}

const packGrid = (items: SizedItem[], tokens: DensityTokens, depth: number): Packed => {
  if (items.length === 0) return { width: 0, height: 0, items: [] }
  const perRow =
    depth === 0
      ? Math.max(tokens.modulesPerRow, Math.ceil(Math.sqrt(items.length * LAYOUT.topLevelRowAspect)))
      : tokens.childrenPerRow
  const gap = tokens.leafGap
  const placed: PlacedItem[] = []
  let x = 0
  let y = 0
  let rowH = 0
  let col = 0
  let maxRowW = 0

  const breakRow = (): void => {
    maxRowW = Math.max(maxRowW, x)
    x = 0
    y += rowH + gap
    rowH = 0
    col = 0
  }

  for (const it of items) {
    const isWide = !it.isLeaf
    const isRowFull = isWide ? col > 0 : col >= perRow
    if (isRowFull) {
      breakRow()
    }
    placed.push({ ...it, x, y })
    x += it.w + gap
    rowH = Math.max(rowH, it.h)
    col += 1
    if (isWide) {
      breakRow()
    }
  }
  maxRowW = Math.max(maxRowW, x)
  return { width: Math.max(maxRowW - gap, 0), height: y + rowH, items: placed }
}

interface SizedContainer {
  packed: Packed
  info: ContainerInfo
}

const sizeContainer = (ctx: LayoutContext, c: DesignContainer): SizedContainer => {
  const { tokens } = ctx
  const tops = ctx.data.topByContainer[c.id] ?? []
  const packed = packGrid(
    tops.map((id) => sizeModule(ctx, id, 0)),
    tokens,
    0,
  )
  const labelH = tokens.containerLabelHeight
  const width = Math.max(
    packed.width + tokens.containerPadding.l + tokens.containerPadding.r,
    LAYOUT.minContainerWidth,
  )
  const height = packed.height + labelH + tokens.containerPadding.b
  return { packed, info: { ...c, width, height, labelH, absX: 0, absY: 0 } }
}

const placeLanes = (
  data: DesignData,
  sized: SizedContainer[],
  tokens: DensityTokens,
): LaneInfo[] => {
  const x = LAYOUT.laneLabelWidth + LAYOUT.laneLabelGap
  let y = 0
  return data.layers.map((layer) => {
    let rowX = 0
    let rowH = 0
    for (const c of sized.filter((s) => s.info.layer === layer.id)) {
      c.info.absX = x + tokens.lanePadX + rowX
      c.info.absY = y + tokens.lanePadY
      rowX += c.info.width + tokens.containerGap
      rowH = Math.max(rowH, c.info.height)
    }
    const lane = { ...layer, x, y, width: rowX + 2 * tokens.lanePadX, height: rowH + 2 * tokens.lanePadY }
    y += lane.height + tokens.laneGapY
    return lane
  })
}

interface EmitTarget {
  out: Record<string, ModulePos>
  tokens: DensityTokens
}

const emitPositions = (
  target: EmitTarget,
  item: PlacedItem,
  origin: { x: number; y: number },
  depth: number,
): void => {
  const x = origin.x + item.x
  const y = origin.y + item.y
  target.out[item.id] = {
    x,
    y,
    w: item.w,
    h: item.h,
    cx: x + item.w / 2,
    cy: y + item.h / 2,
    mod: item.mod,
    isLeaf: item.isLeaf,
    depth,
    childIds: item.placedKids.map((k) => k.id),
  }
  const inner = { x: x + target.tokens.modulePadH, y: y + target.tokens.moduleHeader }
  for (const kid of item.placedKids) emitPositions(target, kid, inner, depth + 1)
}

const positionModules = (
  sized: SizedContainer[],
  tokens: DensityTokens,
): Record<string, ModulePos> => {
  const target: EmitTarget = { out: {}, tokens }
  for (const { packed, info } of sized) {
    const inner = { x: info.absX + tokens.containerPadding.l, y: info.absY + info.labelH }
    for (const item of packed.items) emitPositions(target, item, inner, 0)
  }
  return target.out
}

const layEdges = (data: DesignData, modulePos: Record<string, ModulePos>): LaidEdge[] => {
  const edges: LaidEdge[] = []
  data.edges.forEach((e, i) => {
    const a = modulePos[e.source]
    const b = modulePos[e.target]
    if (!a || !b) return
    edges.push({ ...e, i, x1: a.cx, y1: a.cy, x2: b.cx, y2: b.cy })
  })
  return edges
}

const sceneSize = (lanes: LaneInfo[]): { width: number; height: number } => {
  const last = lanes[lanes.length - 1]
  if (!last) return { width: 0, height: 0 }
  return {
    width: Math.max(...lanes.map((l) => l.x + l.width)) + LAYOUT.sceneMargin,
    height: last.y + last.height + LAYOUT.sceneMargin,
  }
}

export const computeLayout = (data: DesignData, options: LayoutOptions = {}): Scene => {
  const tokens = DENSITY[options.density ?? 'cozy']
  const ctx: LayoutContext = { data, tokens }
  const sized = data.containers.map((c) => sizeContainer(ctx, c))
  const lanes = placeLanes(data, sized, tokens)
  const modulePos = positionModules(sized, tokens)
  return {
    lanes,
    containers: sized.map((c) => c.info),
    modulePos,
    edges: layEdges(data, modulePos),
    ...sceneSize(lanes),
  }
}
