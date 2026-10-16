import { Bounds } from '../../render/viewport'
import { Model } from '../../render/model'
import { Review, ReviewEdge, ReviewNode } from './review'

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

export interface Rect {
  x: number
  y: number
  w: number
  h: number
}

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
}

export interface EdgeMark {
  edge: ReviewEdge
  d: string
  mid: { x: number; y: number }
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

const PARALLEL_SPREAD = 5
const PARALLEL_STEP = 6
const LOOP_DX = 36
const MID_WEIGHT = 0.375
const END_WEIGHT = 0.125
const ACROSS_ROWS = REGION.nodeH * 3

type Geometry = EdgeMark['mid'] & { d: string }

const cubic = (p: readonly number[]): Geometry => {
  const [x0, y0, x1, y1, x2, y2, x3, y3] = p
  return {
    d: `M${x0},${y0} C${x1},${y1} ${x2},${y2} ${x3},${y3}`,
    x: END_WEIGHT * (x0 + x3) + MID_WEIGHT * (x1 + x2),
    y: END_WEIGHT * (y0 + y3) + MID_WEIGHT * (y1 + y2),
  }
}

const edgeGeometry = (a: Rect, b: Rect, salt: number): Geometry => {
  const off = ((salt % PARALLEL_SPREAD) - Math.floor(PARALLEL_SPREAD / 2)) * PARALLEL_STEP
  const ax = a.x + a.w / 2 + off
  const ay = a.y + a.h / 2
  const bx = b.x + b.w / 2 + off
  const by = b.y + b.h / 2
  if (Math.abs(ay - by) > ACROSS_ROWS) {
    const isDown = by > ay
    const y1 = isDown ? a.y + a.h : a.y
    const y2 = isDown ? b.y - 2 : b.y + b.h + 2
    const d = (y2 - y1) / 2
    return cubic([ax, y1, ax, y1 + d, bx, y2 - d, bx, y2])
  }
  if (a.x === b.x) {
    const x = a.x + a.w
    return cubic([x, ay, x + LOOP_DX, ay, x + LOOP_DX, by, x + 2, by])
  }
  const isRight = b.x > a.x
  const x1 = isRight ? a.x + a.w : a.x
  const x2 = isRight ? b.x - 2 : b.x + b.w + 2
  const d = (x2 - x1) / 2
  return cubic([x1, ay, x1 + d, ay, x2 - d, by, x2, by])
}

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

const groupSize = (count: number): { w: number; h: number } => {
  const cols = Math.ceil(count / REGION.maxRows)
  const rows = Math.min(count, REGION.maxRows)
  return {
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
  const width = maxW + REGION.lanePad * 2 + LOOP_DX
  for (const lane of lanes) lane.w = width

  const edges: EdgeMark[] = []
  for (const edge of review.edges) {
    const a = nodes.get(edge.from)
    const b = nodes.get(edge.to)
    if (!a || !b) continue
    const g = edgeGeometry(a, b, Math.abs(edge.from * 7 + edge.to * 13))
    edges.push({ edge, d: g.d, mid: { x: g.x, y: g.y } })
  }

  return {
    lanes,
    slices,
    groups,
    nodes,
    edges,
    bounds: { x: -REGION.lanePad, y: 0, w: width, h: Math.max(0, y - REGION.laneGap) },
  }
}
