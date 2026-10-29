import { SCENE_MARK_BUDGET } from '../../render/budget'
import { Model, Unit } from '../../render/model'
import { Bounds } from '../../render/viewport'
import { MapGate } from '../../shell/nav'
import { SnapshotLayer } from '../../types'
import { isGated, LayerLink, MapData, UnitLink } from './units'

export const BLOCK = {
  pad: 8,
  gap: 5,
  header: 18,
  spacing: 10,
  aspect: 1.5,
  innerAspect: 1.8,
  nameCharPx: 6.4,
  margin: 10,
} as const
export const UNIT_NAME = { charPx: 6.4, pad: 30 } as const
const BOX = { minW: 80, maxW: 220, baseW: 60, perRootW: 6, minH: 34, maxH: 90, baseH: 30, perRootH: 3 } as const
export const GRID = { cell: 13, size: 10, maxCols: 24, aspect: 2.2, top: 42, left: 10, minW: 170, headH: 46, foot: 8 } as const
const CURVE = { maxBend: 80, bendShare: 0.2, arrowClearance: 6 } as const
export const LAYER_NODE = { w: 300, h: 54, gap: 22, row: 17, maxRows: 40, side: 260, bulge: 30, perStep: 18 } as const

export interface Point {
  x: number
  y: number
}

export interface Rect extends Point {
  w: number
  h: number
}

export interface Cell extends Point {
  id: number
}

export interface UnitBox extends Rect {
  unit: Unit
  cells: readonly Cell[] | null
}

/** A layer's frame: the bounding box of its units plus a header, never narrower than `minW` (its name). */
export interface LayerBlock extends Rect {
  layer: number
  units: readonly number[]
  modules: number
  minW: number
}

export interface BlocksLayout {
  blocks: readonly LayerBlock[]
  boxes: ReadonlyMap<number, UnitBox>
  reserved: Rect | null
  bounds: Bounds
}

export interface Curve {
  d: string
  label: Point
}

const gridOf = (modules: readonly number[]): { cols: number; w: number; h: number; cells: Cell[] } => {
  const n = modules.length
  const cols = Math.max(1, Math.min(GRID.maxCols, Math.ceil(Math.sqrt(n * GRID.aspect))))
  const rows = Math.ceil(n / cols)
  const cells = modules.map((id, i) => ({
    id,
    x: GRID.left + (i % cols) * GRID.cell,
    y: GRID.top + Math.floor(i / cols) * GRID.cell,
  }))
  return { cols, w: Math.max(GRID.minW, cols * GRID.cell + GRID.left * 2), h: GRID.headH + rows * GRID.cell + GRID.foot, cells }
}

const boxSize = (unit: Unit, isExpanded: boolean): { w: number; h: number; cells: Cell[] | null } => {
  if (isExpanded) {
    const grid = gridOf(unit.modules)
    return { w: grid.w, h: grid.h, cells: grid.cells }
  }
  const root = Math.sqrt(unit.modules.length)
  return {
    w: Math.max(BOX.minW, Math.min(BOX.maxW, Math.max(BOX.baseW + root * BOX.perRootW, nameWidth(unit.name)))),
    h: Math.max(BOX.minH, Math.min(BOX.maxH, BOX.baseH + root * BOX.perRootH)),
    cells: null,
  }
}

const nameWidth = (name: string): number => Math.ceil(name.length * UNIT_NAME.charPx) + UNIT_NAME.pad

const bySize =(a: Unit, b: Unit): number => b.modules.length - a.modules.length || a.id - b.id

const clip = (from: Point, toward: Point, half: { w: number; h: number }): Point => {
  const dx = toward.x - from.x
  const dy = toward.y - from.y
  const t = Math.min(half.w / Math.abs(dx || Number.EPSILON), half.h / Math.abs(dy || Number.EPSILON), 1)
  return { x: from.x + dx * t, y: from.y + dy * t }
}

const centre = (b: Rect): Point => ({ x: b.x + b.w / 2, y: b.y + b.h / 2 })

export const curveBetween = (a: Rect, b: Rect): Curve => {
  const p = centre(a)
  const q = centre(b)
  const dx = q.x - p.x
  const dy = q.y - p.y
  const len = Math.hypot(dx, dy) || 1
  const bend = Math.min(CURVE.maxBend, len * CURVE.bendShare)
  const control = { x: (p.x + q.x) / 2 - (dy / len) * bend, y: (p.y + q.y) / 2 + (dx / len) * bend }
  const start = clip(p, control, { w: a.w / 2, h: a.h / 2 })
  const end = clip(q, control, { w: (b.w + CURVE.arrowClearance) / 2, h: (b.h + CURVE.arrowClearance) / 2 })
  const r = (n: number) => Math.round(n * 10) / 10
  return {
    d: `M${r(start.x)},${r(start.y)} Q${r(control.x)},${r(control.y)} ${r(end.x)},${r(end.y)}`,
    label: { x: (start.x + 2 * control.x + end.x) / 4, y: (start.y + 2 * control.y + end.y) / 4 },
  }
}

/** Longest chain of `depends_on` below each layer: 0 for a layer that depends on nothing. A cycle is cut where it closes. */
export const layerRanks = (layers: readonly SnapshotLayer[]): ReadonlyMap<number, number> => {
  const byId = new Map(layers.map((l) => [l.id, l]))
  const ranks = new Map<number, number>()
  const onPath = new Set<number>()
  const visit = (id: number): number => {
    const known = ranks.get(id)
    if (known !== undefined) return known
    onPath.add(id)
    let rank = 0
    for (const dep of byId.get(id)?.depends_on ?? []) {
      if (byId.has(dep) && !onPath.has(dep)) rank = Math.max(rank, visit(dep) + 1)
    }
    onPath.delete(id)
    ranks.set(id, rank)
    return rank
  }
  for (const l of layers) visit(l.id)
  return ranks
}

export interface Size {
  w: number
  h: number
}

interface Packed {
  at: Point[]
  w: number
  h: number
}

interface Column {
  x: number
  w: number
  bottom: number
}

const shelfPack = (sizes: readonly Size[], width: number, gap: number): Packed => {
  const at: Point[] = []
  let shelf = { y: 0, h: 0, columns: [] as Column[] }
  let x = 0
  let w = 0
  for (const s of sizes) {
    const under = shelf.columns.find((c) => s.w <= c.w && c.bottom + gap + s.h <= shelf.y + shelf.h)
    if (under) {
      at.push({ x: under.x, y: under.bottom + gap })
      under.bottom += gap + s.h
      continue
    }
    if (at.length && x + s.w > width) {
      shelf = { y: shelf.y + shelf.h + gap, h: 0, columns: [] }
      x = 0
    }
    at.push({ x, y: shelf.y })
    shelf.columns.push({ x, w: s.w, bottom: shelf.y + s.h })
    shelf.h = Math.max(shelf.h, s.h)
    x += s.w + gap
    w = Math.max(w, x - gap)
  }
  return { at, w, h: shelf.y + shelf.h }
}

const WIDTH_TRIES = 48

const packLargestInAspect = (sizes: readonly Size[], aspect: number, gap: number): Packed => {
  const widest = Math.max(...sizes.map((s) => s.w))
  const total = sizes.reduce((sum, s) => sum + s.w + gap, 0)
  let best = shelfPack(sizes, widest, gap)
  const scaleOf = (p: Packed) => Math.min(aspect / p.w, 1 / p.h)
  for (let i = 1; i <= WIDTH_TRIES; i++) {
    const packed = shelfPack(sizes, widest + ((total - widest) * i) / WIDTH_TRIES, gap)
    if (scaleOf(packed) > scaleOf(best)) best = packed
  }
  return best
}

const frameOf = (block: LayerBlock, boxes: ReadonlyMap<number, Rect>): LayerBlock => {
  const rects = block.units.flatMap((id) => boxes.get(id) ?? [])
  const left = Math.min(...rects.map((r) => r.x)) - BLOCK.pad
  const top = Math.min(...rects.map((r) => r.y)) - BLOCK.header
  const right = Math.max(...rects.map((r) => r.x + r.w)) + BLOCK.pad
  const bottom = Math.max(...rects.map((r) => r.y + r.h)) + BLOCK.pad
  return { ...block, x: left, y: top, w: Math.max(right - left, block.minW), h: bottom - top }
}

const boundsOf = (rects: readonly Rect[]): Bounds => {
  if (!rects.length) return { x: 0, y: 0, w: 0, h: 0 }
  const m = BLOCK.margin
  const left = Math.min(...rects.map((r) => r.x)) - m
  const top = Math.min(...rects.map((r) => r.y)) - m
  const right = Math.max(...rects.map((r) => r.x + r.w)) + m
  const bottom = Math.max(...rects.map((r) => r.y + r.h)) + m
  return { x: left, y: top, w: right - left, h: bottom - top }
}

/**
 * One block per layer that has units, ordered by `layerRanks` (the layers that depend on the most come first) and
 * packed to fill a `BLOCK.aspect` rectangle at the largest scale; units are packed inside their block by size.
 * `reserve` is packed last as an empty slot for other marks and comes back as `reserved`.
 */
export const layoutBlocks = (m: Model, expanded: ReadonlySet<number>, reserve: Size | null = null): BlocksLayout => {
  const ranks = layerRanks(m.layers)
  const layers = m.layers
    .map((layer) => ({ layer, units: m.units.filter((u) => u.layer === layer.id).sort(bySize) }))
    .filter((l) => l.units.length)
    .sort(
      (a, b) =>
        (ranks.get(b.layer.id) ?? 0) - (ranks.get(a.layer.id) ?? 0) ||
        a.layer.name.localeCompare(b.layer.name) ||
        a.layer.id - b.layer.id,
    )
  const inner = layers.map(({ layer, units }) => {
    const sizes = units.map((u) => boxSize(u, expanded.has(u.id)))
    const packed = packLargestInAspect(sizes, BLOCK.innerAspect, BLOCK.gap)
    const modules = units.reduce((n, u) => n + u.modules.length, 0)
    const minW = layer.name.length * BLOCK.nameCharPx + BLOCK.pad * 2
    return { layer, units, sizes, packed, minW, modules }
  })
  const outer = inner.map((b) => ({
    w: Math.max(b.packed.w + BLOCK.pad * 2, b.minW),
    h: b.packed.h + BLOCK.header + BLOCK.pad,
  }))
  const placed = packLargestInAspect(reserve ? [...outer, reserve] : outer, BLOCK.aspect, BLOCK.spacing)
  const boxes = new Map<number, UnitBox>()
  const blocks = inner.map((b, i): LayerBlock => {
    const origin = { x: placed.at[i].x + BLOCK.pad, y: placed.at[i].y + BLOCK.header }
    b.units.forEach((unit, j) => {
      boxes.set(unit.id, { unit, ...b.sizes[j], x: origin.x + b.packed.at[j].x, y: origin.y + b.packed.at[j].y })
    })
    const block = {
      layer: b.layer.id,
      units: b.units.map((u) => u.id),
      modules: b.modules,
      minW: b.minW,
    }
    return frameOf({ ...block, x: 0, y: 0, w: 0, h: 0 }, boxes)
  })
  const reserved = reserve ? { ...placed.at[inner.length], ...reserve } : null
  return { blocks, boxes, reserved, bounds: boundsOf(reserved ? [...blocks, reserved] : blocks) }
}

/** The layout with the given units moved to a new top-left corner; each block's frame follows its units. */
export const moveUnits = (layout: BlocksLayout, at: ReadonlyMap<number, Point>): BlocksLayout => {
  if (!at.size) return layout
  const boxes = new Map(layout.boxes)
  for (const [id, p] of at) {
    const box = boxes.get(id)
    if (box) boxes.set(id, { ...box, x: p.x, y: p.y })
  }
  const blocks = layout.blocks.map((b) => (b.units.some((id) => at.has(id)) ? frameOf(b, boxes) : b))
  const reserved = layout.reserved
  return { blocks, boxes, reserved, bounds: boundsOf(reserved ? [...blocks, reserved] : blocks) }
}

const BLOCK_MARKS = 4
const UNIT_MARKS = 2

export interface DefaultLinks {
  links: readonly UnitLink[]
  hidden: number
}

/**
 * The links drawn while no unit is hovered or selected. Past `SCENE_MARK_BUDGET` the gate narrows to rule breaks
 * plus the expanded units' links, then to rule breaks alone; the expanded cells themselves always stay.
 */
export const defaultLinks = (
  layout: BlocksLayout,
  data: MapData,
  gate: MapGate,
  expanded: ReadonlySet<number>,
  pairKey: string | null,
): DefaultLinks => {
  let base = layout.blocks.length * BLOCK_MARKS
  for (const b of layout.boxes.values()) {
    base += UNIT_MARKS + Number((data.units.get(b.unit.id)?.errors ?? 0) > 0) + (b.cells?.length ?? 0)
  }
  const isPair = (l: UnitLink) => l.key === pairKey
  const isBreak = (l: UnitLink) => l.breaks > 0 || isPair(l)
  const drawn = data.links.filter((l) => layout.boxes.has(l.from) && layout.boxes.has(l.to))
  const gated = drawn.filter((l) => isGated(l, gate) || isPair(l))
  const tiers = [
    gated,
    gated.filter((l) => isBreak(l) || expanded.has(l.from) || expanded.has(l.to)),
    gated.filter(isBreak),
  ]
  const marks = (links: readonly UnitLink[]) => base + links.length + Number(links.some(isPair))
  const links = tiers.find((t) => marks(t) <= SCENE_MARK_BUDGET) ?? tiers[tiers.length - 1]
  return { links, hidden: gated.length - links.length }
}

export interface LayerNode {
  layer: number
  y: number
  h: number
  units: readonly Unit[]
  modules: number
}

export interface LayerCurve extends Curve {
  link: LayerLink
}

export interface LayerGraphLayout {
  nodes: readonly LayerNode[]
  curves: readonly LayerCurve[]
  bounds: Bounds
}

export const layoutLayers = (m: Model, data: MapData, open: number | null): LayerGraphLayout => {
  const L = LAYER_NODE
  const nodes: LayerNode[] = []
  let y = 0
  for (const layer of m.layers) {
    const units = m.units.filter((u) => u.layer === layer.id).sort(bySize)
    const h = open === layer.id ? L.h + Math.min(units.length, L.maxRows) * L.row + GRID.foot : L.h
    nodes.push({ layer: layer.id, y, h, units, modules: units.reduce((n, u) => n + u.modules.length, 0) })
    y += h + L.gap
  }
  const byLayer = new Map(nodes.map((n, i) => [n.layer, { node: n, index: i }]))
  const curves = data.layerLinks.flatMap((link): LayerCurve[] => {
    const a = byLayer.get(link.from)
    const b = byLayer.get(link.to)
    if (!a || !b) return []
    const side = link.isDeclared ? -1 : 1
    const x0 = link.isDeclared ? 0 : L.w
    const y1 = a.node.y + L.h / 2
    const y2 = b.node.y + L.h / 2
    const bulge = side * (L.bulge + Math.abs(b.index - a.index) * L.perStep)
    return [
      {
        link,
        d: `M${x0},${y1} C${x0 + bulge},${y1} ${x0 + bulge},${y2} ${x0 + side * 2},${y2}`,
        label: { x: x0 + bulge * 0.75, y: (y1 + y2) / 2 },
      },
    ]
  })
  return { nodes, curves, bounds: { x: -L.side, y: -L.gap, w: L.w + L.side * 2, h: y + L.gap } }
}
