import { Bounds } from '../../render/viewport'
import { Model } from '../../render/model'
import { curveBetween, GridLine, Point, Rect, routeAround, RouteLink } from '../../render/route'
import { Review, ReviewEdge, ReviewNode, worstBreak } from './review'

export const REGION = {
  nodeW: 184,
  nodeH: 22,
  nodeGap: 6,
  groupPad: 8,
  groupHead: 24,
  groupGap: 12,
  sliceHead: 22,
  sliceGap: 24,
  laneHead: 26,
  lanePad: 14,
  laneGap: 60,
  laneWrap: 2600,
  maxRows: 14,
} as const

export interface LaneBox extends Rect {
  layer: number | null
}

export interface SliceBox extends Rect {
  container: number
}

export interface GroupBox extends Rect {
  key: string
  name: string
  container: number | null
  isPrivate: boolean
  layer: number | null
  cols: number
  rows: number
}

export interface EdgeMark {
  edge: ReviewEdge
  d: string
  mid: Point
}

export interface RegionLayout {
  lanes: LaneBox[]
  slices: SliceBox[]
  groups: GroupBox[]
  nodes: Map<number, Rect>
  edges: EdgeMark[]
  bounds: Bounds
}

interface Block {
  slice: number | null
  groups: { node: ReviewNode; ids: ReviewNode[] }[]
}

const MARGIN_X = 36
const HEAD_LANE = 4
const SLICE_LANE = 3
const CHAR_PX = 6
const META_CHAR_PX = 5.6
const ROUTE_BEND = 220
const ROUTE_TRACK = 2.5
const ROUTE_SIDES = ['left', 'right'] as const
const WEIGHT = { breaking: 3, changed: 2, same: 1 } as const

const routingLines = (groups: readonly GroupBox[], slices: readonly SliceBox[], lanes: readonly LaneBox[], right: number) => {
  const xs: GridLine[] = [
    { at: -REGION.lanePad / 2, cost: 1, room: REGION.lanePad / 2 - 2 },
    { at: right - MARGIN_X / 2, cost: 1, room: MARGIN_X / 2 - 2 },
  ]
  const ys: GridLine[] = []
  const outer = { cost: 1, room: REGION.groupGap / 2 - 2 }
  const inner = { cost: 1.2, room: REGION.groupPad / 2 - 2 }
  const between = { cost: 1.2, room: REGION.nodeGap / 2 - 1.5 }
  for (const g of groups) {
    xs.push({ at: g.x - REGION.groupGap / 2, ...outer }, { at: g.x + g.w + REGION.groupGap / 2, ...outer })
    xs.push({ at: g.x + REGION.groupPad / 2, ...inner }, { at: g.x + g.w - REGION.groupPad / 2, ...inner })
    for (let k = 1; k < g.cols; k++) {
      xs.push({ at: g.x + REGION.groupPad + k * (REGION.nodeW + REGION.nodeGap) - REGION.nodeGap / 2, ...between })
    }
    ys.push({ at: g.y - SLICE_LANE, cost: 1.2, room: 1 }, { at: g.y + g.h + REGION.groupGap / 2, ...outer })
    ys.push({ at: g.y + REGION.groupHead - HEAD_LANE, cost: 1.3, room: 1.5 })
    for (let k = 1; k < g.rows; k++) {
      ys.push({ at: g.y + REGION.groupHead + k * (REGION.nodeH + REGION.nodeGap) - REGION.nodeGap / 2, cost: 3, room: 1 })
    }
  }
  const slicing = { cost: 1, room: REGION.sliceGap / 2 - 2 }
  for (const s of slices) {
    xs.push({ at: s.x - REGION.sliceGap / 2, ...slicing }, { at: s.x + s.w + REGION.sliceGap / 2, ...slicing })
    ys.push({ at: s.y + s.h + REGION.sliceGap / 2, ...slicing })
  }
  const laning = { cost: 1, room: REGION.laneGap / 2 - 4 }
  for (const l of lanes) {
    ys.push({ at: l.y - REGION.laneGap / 2, ...laning }, { at: l.y + l.h + REGION.laneGap / 2, ...laning })
  }
  return { xs, ys }
}

export const GROUP_CHARS = 26

export const stageText = (review: Review, container: number | null, fallback: number | undefined): string => {
  const change = container === null ? undefined : review.stageOf.get(container)
  if (change) return change.base === null ? `→ s${change.head}` : `s${change.base} → s${change.head}`
  return fallback ? `s${fallback}` : ''
}

export const groupMeta = (m: Model, review: Review, g: GroupBox): string => {
  if (g.isPrivate) return 'private'
  return stageText(review, g.container, g.container === null ? undefined : m.containers.get(g.container)?.stage)
}

const headRects = (g: GroupBox, meta: string): Rect[] => {
  const name = Math.min(g.name.length, GROUP_CHARS)
  const metaW = meta.length * META_CHAR_PX + 4
  return [
    { x: g.x + 6, y: g.y + 5, w: name * CHAR_PX + 6, h: REGION.groupHead - HEAD_LANE - 7 },
    { x: g.x + g.w - 9 - metaW, y: g.y + 5, w: metaW, h: REGION.groupHead - HEAD_LANE - 7 },
  ]
}

const breakStyle = (e: ReviewEdge): string => worstBreak(e.breaks) ?? ''

const weightOf = (e: ReviewEdge): number =>
  e.breaks.length ? WEIGHT.breaking : e.tag === 'same' ? WEIGHT.same : WEIGHT.changed

const byPath = (a: string, b: string): number => a.length - b.length || a.localeCompare(b)

const blocksOf = (nodes: readonly ReviewNode[]): Block[] => {
  const blocks = new Map<string, Block>()
  for (const n of nodes) {
    const key = n.slice === null ? `g${n.group.key}` : `s${n.slice}`
    const block = blocks.get(key) ?? { slice: n.slice, groups: [] }
    blocks.set(key, block)
    const group = block.groups.find((g) => g.node.group.key === n.group.key)
    if (group) {
      group.ids.push(n)
    } else {
      block.groups.push({ node: n, ids: [n] })
    }
  }
  const list = [...blocks.values()]
  for (const b of list) {
    b.groups.sort((x, y) => byPath(x.node.group.path, y.node.group.path))
    for (const g of b.groups) g.ids.sort((x, y) => x.path.localeCompare(y.path))
  }
  return list.sort((x, y) => x.groups[0].node.group.path.localeCompare(y.groups[0].node.group.path))
}

const groupSize = (count: number): { w: number; h: number; cols: number; rows: number } => {
  const cols = Math.ceil(count / REGION.maxRows)
  const rows = Math.min(count, REGION.maxRows)
  return {
    cols,
    rows,
    w: cols * (REGION.nodeW + REGION.nodeGap) - REGION.nodeGap + REGION.groupPad * 2,
    h: REGION.groupHead + rows * (REGION.nodeH + REGION.nodeGap) + REGION.groupPad / 2,
  }
}

const nodeRect = (gx: number, gy: number, index: number): Rect => {
  const col = Math.floor(index / REGION.maxRows)
  const row = index % REGION.maxRows
  return {
    x: gx + REGION.groupPad + col * (REGION.nodeW + REGION.nodeGap),
    y: gy + REGION.groupHead + row * (REGION.nodeH + REGION.nodeGap),
    w: REGION.nodeW,
    h: REGION.nodeH,
  }
}

export const layoutRegion = (m: Model, review: Review): RegionLayout => {
  const order = new Map(m.layers.map((l, i) => [l.id, i]))
  const laneIndex = (n: ReviewNode): number => (n.layer === null ? order.size : (order.get(n.layer) ?? order.size))
  const byLane = new Map<number, ReviewNode[]>()
  for (const n of review.nodes.values()) {
    const key = laneIndex(n)
    byLane.set(key, [...(byLane.get(key) ?? []), n])
  }

  const lanes: LaneBox[] = []
  const slices: SliceBox[] = []
  const groups: GroupBox[] = []
  const nodes = new Map<number, Rect>()
  let y = 0
  let maxW = 0
  for (const key of [...byLane.keys()].sort((a, b) => a - b)) {
    const top = y
    let x = 0
    let rowTop = top + REGION.laneHead
    let rowH = 0
    let laneW = 0
    for (const block of blocksOf(byLane.get(key)!)) {
      const sizes = block.groups.map((g) => groupSize(g.ids.length))
      const blockW = sizes.reduce<number>((sum, s) => sum + s.w + REGION.groupGap, REGION.groupGap)
      const blockH = REGION.sliceHead + Math.max(...sizes.map((s) => s.h)) + REGION.groupGap
      if (x > 0 && x + blockW > REGION.laneWrap) {
        x = 0
        rowTop += rowH + REGION.sliceGap
        rowH = 0
      }
      if (block.slice !== null) slices.push({ container: block.slice, x, y: rowTop, w: blockW, h: blockH })
      let gx = x + REGION.groupGap
      const gy = rowTop + REGION.sliceHead
      block.groups.forEach((g, i) => {
        const { node } = g
        groups.push({ ...node.group, layer: node.layer, x: gx, y: gy, ...sizes[i] })
        g.ids.forEach((n, j) => nodes.set(n.id, nodeRect(gx, gy, j)))
        gx += sizes[i].w + REGION.groupGap
      })
      x += blockW + REGION.sliceGap
      laneW = Math.max(laneW, x - REGION.sliceGap)
      rowH = Math.max(rowH, blockH)
    }
    const laneH = rowTop + rowH + REGION.lanePad - top
    lanes.push({ layer: m.layers[key]?.id ?? null, x: -REGION.lanePad, y: top, w: 0, h: laneH })
    maxW = Math.max(maxW, laneW)
    y = top + laneH + REGION.laneGap
  }
  const width = maxW + REGION.lanePad * 2 + MARGIN_X
  for (const lane of lanes) lane.w = width

  const groupOf = new Map<number, string>()
  for (const n of review.nodes.values()) groupOf.set(n.id, n.group.key)
  const drawn = review.edges.filter((e) => nodes.has(e.from) && nodes.has(e.to))
  const links = drawn.map(
    (e): RouteLink => ({
      key: e.key,
      from: nodes.get(e.from)!,
      to: nodes.get(e.to)!,
      weight: weightOf(e),
      sides: ROUTE_SIDES,
      bundle: `${groupOf.get(e.from)}>${groupOf.get(e.to)}:${e.tag}:${breakStyle(e)}:${e.kind}`,
    }),
  )
  const obstacles = [...nodes.values(), ...groups.flatMap((g) => headRects(g, groupMeta(m, review, g)))]
  const routes = routeAround(links, {
    obstacles,
    lines: routingLines(groups, slices, lanes, width - REGION.lanePad),
    bend: ROUTE_BEND,
    track: ROUTE_TRACK,
  })
  const edges = drawn.map((edge): EdgeMark => {
    const route = routes.get(edge.key) ?? curveBetween(nodes.get(edge.from)!, nodes.get(edge.to)!)
    return { edge, d: route.d, mid: route.label }
  })

  return {
    lanes,
    slices,
    groups,
    nodes,
    edges,
    bounds: { x: -REGION.lanePad, y: 0, w: width, h: Math.max(0, y - REGION.laneGap) },
  }
}
