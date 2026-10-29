import { SCENE_MARK_BUDGET } from '../../render/budget'
import { Model, Unit } from '../../render/model'
import { Bounds } from '../../render/viewport'
import { MapGate } from '../../shell/nav'
import { isGated, LayerLink, MapData, UnitLink } from './units'

export const LANES = { width: 1700, pad: 14, gap: 10, header: 26, spacing: 12, margin: 10 } as const
const BOX = { minW: 104, maxW: 240, baseW: 70, perRootW: 9, minH: 40, maxH: 110, baseH: 28, perRootH: 4.5 } as const
export const GRID = { cell: 13, size: 10, maxCols: 24, aspect: 2.2, top: 42, left: 10, minW: 170, headH: 46, foot: 8 } as const
const CURVE = { maxBend: 80, bendShare: 0.2, arrowClearance: 6 } as const
export const LAYER_NODE = { w: 300, h: 54, gap: 22, row: 17, maxRows: 40, side: 260, bulge: 30, perStep: 18 } as const

export interface Point {
  x: number
  y: number
}

export interface Cell extends Point {
  id: number
}

export interface UnitBox {
  unit: Unit
  x: number
  y: number
  w: number
  h: number
  cells: readonly Cell[] | null
}

export interface Lane {
  layer: number
  y: number
  h: number
  modules: number
}

export interface Curve {
  link: UnitLink
  d: string
  label: Point
}

export interface LanesLayout {
  lanes: readonly Lane[]
  boxes: ReadonlyMap<number, UnitBox>
  curves: readonly Curve[]
  bounds: Bounds
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
    w: Math.max(BOX.minW, Math.min(BOX.maxW, BOX.baseW + root * BOX.perRootW)),
    h: Math.max(BOX.minH, Math.min(BOX.maxH, BOX.baseH + root * BOX.perRootH)),
    cells: null,
  }
}

const bySize = (a: Unit, b: Unit): number => b.modules.length - a.modules.length || a.id - b.id

const clip = (from: Point, toward: Point, half: { w: number; h: number }): Point => {
  const dx = toward.x - from.x
  const dy = toward.y - from.y
  const t = Math.min(half.w / Math.abs(dx || Number.EPSILON), half.h / Math.abs(dy || Number.EPSILON), 1)
  return { x: from.x + dx * t, y: from.y + dy * t }
}

export interface Rect extends Point {
  w: number
  h: number
}

const centre = (b: Rect): Point => ({ x: b.x + b.w / 2, y: b.y + b.h / 2 })

export const curveBetween = (a: Rect, b: Rect): { d: string; label: Point } => {
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

export const layoutLanes = (m: Model, data: MapData, expanded: ReadonlySet<number>): LanesLayout => {
  const boxes = new Map<number, UnitBox>()
  const lanes: Lane[] = []
  let top = 0
  for (const layer of m.layers) {
    const units = m.units.filter((u) => u.layer === layer.id).sort(bySize)
    if (!units.length) continue
    let x: number = LANES.pad
    let y = top + LANES.header
    let rowH = 0
    for (const unit of units) {
      const size = boxSize(unit, expanded.has(unit.id))
      if (x + size.w > LANES.width - LANES.pad && x > LANES.pad) {
        x = LANES.pad
        y += rowH + LANES.gap
        rowH = 0
      }
      boxes.set(unit.id, { unit, x, y, ...size })
      x += size.w + LANES.gap
      rowH = Math.max(rowH, size.h)
    }
    const h = y + rowH + LANES.pad - top
    lanes.push({ layer: layer.id, y: top, h, modules: units.reduce((n, u) => n + u.modules.length, 0) })
    top += h + LANES.spacing
  }
  const curves = data.links.flatMap((link) => {
    const a = boxes.get(link.from)
    const b = boxes.get(link.to)
    return a && b ? [{ link, ...curveBetween(a, b) }] : []
  })
  const margin = LANES.margin
  return { lanes, boxes, curves, bounds: { x: -margin, y: -margin, w: LANES.width + margin * 2, h: top + margin } }
}

const LANE_MARKS = 4
const UNIT_MARKS = 2

export interface DefaultCurves {
  curves: readonly Curve[]
  hidden: number
}

/**
 * The curves drawn while no unit is hovered or selected. Past `SCENE_MARK_BUDGET` the gate narrows to rule breaks
 * plus the expanded units' links, then to rule breaks alone; the expanded cells themselves always stay.
 */
export const defaultCurves = (
  layout: LanesLayout,
  data: MapData,
  gate: MapGate,
  expanded: ReadonlySet<number>,
  pairKey: string | null,
): DefaultCurves => {
  let base = layout.lanes.length * LANE_MARKS
  for (const b of layout.boxes.values()) {
    base += UNIT_MARKS + Number((data.units.get(b.unit.id)?.errors ?? 0) > 0) + (b.cells?.length ?? 0)
  }
  const isPair = (c: Curve) => c.link.key === pairKey
  const isBreak = (c: Curve) => c.link.breaks > 0 || isPair(c)
  const gated = layout.curves.filter((c) => isGated(c.link, gate) || isPair(c))
  const tiers = [
    gated,
    gated.filter((c) => isBreak(c) || expanded.has(c.link.from) || expanded.has(c.link.to)),
    gated.filter(isBreak),
  ]
  const marks = (curves: readonly Curve[]) => base + curves.length + Number(curves.some(isPair))
  const curves = tiers.find((t) => marks(t) <= SCENE_MARK_BUDGET) ?? tiers[tiers.length - 1]
  return { curves, hidden: gated.length - curves.length }
}

export interface LayerNode {
  layer: number
  y: number
  h: number
  units: readonly Unit[]
  modules: number
}

export interface LayerCurve {
  link: LayerLink
  d: string
  label: Point
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
