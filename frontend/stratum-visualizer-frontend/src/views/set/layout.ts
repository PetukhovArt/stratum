import { SeverityLabel } from '../../render/encoding'
import { Model } from '../../render/model'
import { Bounds } from '../../render/viewport'
import { EdgeKind } from '../../types'
import { bestCut, pairKey, SetAnalysis } from './analysis'

export const RING_NODE = { w: 184, h: 30, viaH: 24 } as const
export const ARC_NODE = { w: 120, h: 22, spacing: 132 } as const
export const SET_CELL = 22
export const MATRIX_LABEL_SPACE = 170

const RING = { minRadius: 130, perMember: 34, viaGap: 190, viaStagger: 36 } as const
const RING_BEND = { single: 12, both: 26 } as const
const ARROW_GAP = 3
const ARC = { maxHeight: 320, perSpan: 40, base: 12, startY: 12, endY: 14 } as const
const BEZIER_PEAK = 0.75

export interface SetLink {
  key: string
  from: number
  to: number
  kind: EdgeKind
  isPath: boolean
  isChain: boolean
  isCut: boolean
  rules: string
  worst: SeverityLabel | null
}

export interface SetNode {
  id: number
  x: number
  y: number
  isMember: boolean
  isLoop: boolean
}

export interface CurvedLink extends SetLink {
  d: string
  badge: { x: number; y: number }
}

export interface CurveScene {
  nodes: SetNode[]
  links: CurvedLink[]
  bounds: Bounds
}

export interface SetCell extends SetLink {
  row: number
  col: number
  isAbove: boolean
}

export interface SetMatrix {
  order: readonly number[]
  members: ReadonlySet<number>
  cells: SetCell[]
  /** Index range of the loop in `order`, inclusive. */
  block: { start: number; end: number } | null
  bounds: Bounds
}

const chainKeys = (a: SetAnalysis): Set<string> => {
  const keys = new Set<string>()
  const chain = a.chain ?? []
  for (let i = 0; i < chain.length - 1; i++) keys.add(pairKey(chain[i], chain[i + 1]))
  return keys
}

const linksOf = (m: Model, a: SetAnalysis): SetLink[] => {
  const chain = chainKeys(a)
  const cut = bestCut(a)
  const cutKey = cut ? pairKey(cut.from, cut.to) : null
  return a.edges.map((e) => {
    const key = pairKey(e.from, e.to)
    const findings = m.findingsOnEdge(e.from, e.to)
    return {
      key,
      from: e.from,
      to: e.to,
      kind: m.edge(e.from, e.to)?.kind ?? EdgeKind.Static,
      isPath: e.isPath,
      isChain: chain.has(key),
      isCut: key === cutKey,
      rules: [...new Set(findings.map((v) => v.rule))].sort().join(' '),
      worst: m.worst(findings),
    }
  })
}

const unionBounds = (boxes: readonly Bounds[]): Bounds => {
  if (!boxes.length) return { x: 0, y: 0, w: 0, h: 0 }
  const x = Math.min(...boxes.map((b) => b.x))
  const y = Math.min(...boxes.map((b) => b.y))
  const right = Math.max(...boxes.map((b) => b.x + b.w))
  const bottom = Math.max(...boxes.map((b) => b.y + b.h))
  return { x, y, w: right - x, h: bottom - y }
}

const point = (p: { x: number; y: number }): Bounds => ({ x: p.x, y: p.y, w: 0, h: 0 })

const clip = (x1: number, y1: number, x2: number, y2: number, hw: number, hh: number): [number, number] => {
  const dx = x2 - x1
  const dy = y2 - y1
  const t = Math.min(hw / Math.abs(dx || 1e-6), hh / Math.abs(dy || 1e-6), 1)
  return [x1 + dx * t, y1 + dy * t]
}

const ringNodes = (m: Model, a: SetAnalysis): SetNode[] => {
  const loop = new Set(a.loop ?? [])
  const members = new Set(a.members)
  const ringIds = a.order.filter((id) => members.has(id))
  const radius = Math.max(RING.minRadius, ringIds.length * RING.perMember)
  const angle = new Map<number, number>()
  const nodes: SetNode[] = ringIds.map((id, i) => {
    const t = -Math.PI / 2 + (i * 2 * Math.PI) / ringIds.length
    angle.set(id, t)
    return { id, x: Math.cos(t) * radius, y: Math.sin(t) * radius, isMember: true, isLoop: loop.has(id) }
  })
  const outer = radius + RING.viaGap
  a.order
    .filter((id) => !members.has(id))
    .forEach((id, i) => {
      const near = [...m.out(id), ...m.inn(id)].filter((x) => angle.has(x))
      const sin = near.reduce((s, x) => s + Math.sin(angle.get(x)!), 0)
      const cos = near.reduce((s, x) => s + Math.cos(angle.get(x)!), 0)
      const t = near.length ? Math.atan2(sin, cos) : i
      const y = Math.sin(t) * outer + (i % 2) * RING.viaStagger
      nodes.push({ id, x: Math.cos(t) * outer, y, isMember: false, isLoop: false })
    })
  return nodes
}

const nodeBox = (n: SetNode, w: number, h: number): Bounds => ({ x: n.x - w / 2, y: n.y - h / 2, w, h })

export const layoutRing = (m: Model, a: SetAnalysis): CurveScene => {
  const nodes = ringNodes(m, a)
  const at = new Map(nodes.map((n) => [n.id, n]))
  const keys = new Set(a.edges.map((e) => pairKey(e.from, e.to)))
  const hw = RING_NODE.w / 2
  const hh = RING_NODE.h / 2
  const links = linksOf(m, a).map((l): CurvedLink => {
    const p = at.get(l.from)!
    const q = at.get(l.to)!
    const len = Math.hypot(q.x - p.x, q.y - p.y) || 1
    const bend = keys.has(pairKey(l.to, l.from)) ? RING_BEND.both : RING_BEND.single
    const cx = (p.x + q.x) / 2 - ((q.y - p.y) / len) * bend
    const cy = (p.y + q.y) / 2 + ((q.x - p.x) / len) * bend
    const [x1, y1] = clip(p.x, p.y, cx, cy, hw, hh)
    const [x2, y2] = clip(q.x, q.y, cx, cy, hw + ARROW_GAP, hh + ARROW_GAP)
    return { ...l, d: `M${x1},${y1} Q${cx},${cy} ${x2},${y2}`, badge: { x: cx, y: cy } }
  })
  const bounds = unionBounds([
    ...nodes.map((n) => nodeBox(n, RING_NODE.w, n.isMember ? RING_NODE.h : RING_NODE.viaH)),
    ...links.map((l) => point(l.badge)),
  ])
  return { nodes, links, bounds }
}

export const layoutArc = (m: Model, a: SetAnalysis): CurveScene => {
  const loop = new Set(a.loop ?? [])
  const members = new Set(a.members)
  const index = new Map(a.order.map((id, i) => [id, i]))
  const nodes: SetNode[] = a.order.map((id, i) => ({
    id,
    x: i * ARC_NODE.spacing,
    y: 0,
    isMember: members.has(id),
    isLoop: loop.has(id),
  }))
  const base = linksOf(m, a)
  const maxSpan = Math.max(1, ...base.map((l) => Math.abs(index.get(l.to)! - index.get(l.from)!)))
  const height = Math.min(ARC.maxHeight, maxSpan * ARC.perSpan)
  const links = base.map((l): CurvedLink => {
    const i = index.get(l.from)!
    const j = index.get(l.to)!
    const sign = j < i ? -1 : 1
    const y = sign * ((Math.abs(j - i) / maxSpan) * height + ARC.base)
    const x1 = i * ARC_NODE.spacing
    const x2 = j * ARC_NODE.spacing
    const d = `M${x1},${sign * ARC.startY} C${x1},${y} ${x2},${y} ${x2},${sign * ARC.endY}`
    return { ...l, d, badge: { x: (x1 + x2) / 2, y: y * BEZIER_PEAK } }
  })
  const bounds = unionBounds([
    ...nodes.map((n) => nodeBox(n, ARC_NODE.w, ARC_NODE.h)),
    ...links.map((l) => point(l.badge)),
  ])
  return { nodes, links, bounds }
}

export const layoutSetMatrix = (m: Model, a: SetAnalysis): SetMatrix => {
  const index = new Map(a.order.map((id, i) => [id, i]))
  const cells = linksOf(m, a).map((l): SetCell => {
    const row = index.get(l.from)!
    const col = index.get(l.to)!
    return { ...l, row, col, isAbove: col > row }
  })
  const loopIdx = (a.loop ?? []).map((id) => index.get(id)!)
  const block = loopIdx.length ? { start: Math.min(...loopIdx), end: Math.max(...loopIdx) } : null
  const span = a.order.length * SET_CELL
  return {
    order: a.order,
    members: new Set(a.members),
    cells,
    block,
    bounds: { x: -MATRIX_LABEL_SPACE, y: -MATRIX_LABEL_SPACE, w: MATRIX_LABEL_SPACE + span, h: MATRIX_LABEL_SPACE + span },
  }
}
