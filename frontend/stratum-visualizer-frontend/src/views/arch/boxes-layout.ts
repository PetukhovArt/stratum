import { alongRun, Point, Rect, roundedPath } from '../../render/route'
import { Bounds } from '../../render/viewport'
import { SnapshotLayer } from '../../types'
import { Box, BoxLink, Boxes } from './boxes'

export type Orient = 'rows' | 'columns'

interface Axis {
  box: { o: number; r: number }
  port: { o: number; r: number }
  portStep: number
  channel: number
  label: number
  boxGap: number
}

const AXIS: Readonly<Record<Orient, Axis>> = {
  rows: { box: { o: 196, r: 58 }, port: { o: 170, r: 46 }, portStep: 16, channel: 64, label: 14, boxGap: 36 },
  columns: { box: { o: 58, r: 196 }, port: { o: 46, r: 170 }, portStep: 12, channel: 96, label: 18, boxGap: 44 },
}
const PER_ROW = [5, 6, 7] as const
const GAP = { dummy: 12, dummies: 8, lanePad: 6, lane: 8, edge: 22, track: 8, minTrack: 3 } as const
const CHIP = { gap: 12, h: 18, between: 4, charPx: 6.2, pad: 14, maxW: 132, minW: 56 } as const
const HUB = { minIn: 6, share: 0.4, ranksBelow: 2 } as const
const SCORE = { crossing: 10, corner: 3, length: 0.01, imbalance: 5, aspect: 40 } as const
const SIDE_PAD = 10
const SIDE_GROWTH = 2
const CORNER = 9
const SWEEPS = 6
const TRANSPOSE_PASSES = 4
const PLACE_PASSES = 6
const MARGIN = 40
const TOP_LINKS = 16
const STRAIGHT = 1.5
const SNAP = 8
const BUNDLE_MIN = 3

export interface PlacedBox extends Rect {
  box: Box
  usedBy: number
}

export interface PlacedLink {
  link: BoxLink
  d: string
  label: Point
}

export interface PortTarget {
  id: string
  name: string
  isExternal: boolean
}

export interface PortFlow {
  box: string
  port: string
  weight: number
}

export interface PlacedPort extends Rect {
  port: PortTarget
}

export interface PlacedFlow {
  flow: PortFlow
  d: string
  label: Point
}

export interface PlacedStub extends Rect {
  link: BoxLink
  hubName: string
  d: string
}

export interface AreaFrame extends Rect {
  name: string
}

interface Item {
  key: string
  row: number
  o: number
  r: number
  size: number
  extra: number
  depth: number
  isReal: boolean
  gap: number
  area: string
}

interface Gap {
  lo: number
  hi: number
}

interface Row {
  items: Item[]
  gaps: Gap[]
}

interface Span {
  start: number
  end: number
}

interface Frame {
  orient: Orient
  rows: Row[]
  channels: Span[]
  items: ReadonlyMap<string, Item>
  dummies: ReadonlyMap<string, number>
}

interface Wire {
  key: string
  from: string
  to: string
  weight: number
  link?: BoxLink
  flow?: PortFlow
}

interface Plan {
  ports: Map<string, number>
  passages: Map<string, number>
  tracks: Map<string, number>
  counts: number[]
  fanOut: Map<string, string>
  fanIn: Map<string, string>
  bundles: readonly Segment[]
}

export interface BoxesLayout {
  boxes: readonly PlacedBox[]
  byKey: ReadonlyMap<string, PlacedBox>
  links: readonly BoxLink[]
  stubbed: ReadonlySet<string>
  stubs: readonly PlacedStub[]
  areas: readonly AreaFrame[]
  ports: readonly PlacedPort[]
  flows: readonly PortFlow[]
  bounds: Bounds
  frame: Frame
}

const ranksOf = (boxes: readonly Box[], links: readonly BoxLink[]): Map<string, number> => {
  const out = new Map<string, string[]>(boxes.map((b) => [b.key, []]))
  for (const l of links) out.get(l.from)?.push(l.to)
  const ranks = new Map<string, number>()
  const onPath = new Set<string>()
  const visit = (key: string): number => {
    const known = ranks.get(key)
    if (known !== undefined) return known
    if (onPath.has(key)) return 0
    onPath.add(key)
    const rank = Math.max(-1, ...(out.get(key) ?? []).map(visit)) + 1
    onPath.delete(key)
    ranks.set(key, rank)
    return rank
  }
  for (const b of boxes) visit(b.key)
  return ranks
}

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

const layerRanksOf = (boxes: readonly Box[], layerRanks: ReadonlyMap<number, number>): Map<string, number> | null => {
  const ranks = new Map<string, number>()
  for (const b of boxes) {
    const rank = b.layer === null ? undefined : layerRanks.get(b.layer)
    if (rank === undefined) return null
    ranks.set(b.key, rank)
  }
  return new Set(ranks.values()).size > 1 ? ranks : null
}

const byName = (a: Box, b: Box): number => (a.name < b.name ? -1 : a.name > b.name ? 1 : a.key < b.key ? -1 : 1)

const rowsOf = (
  boxes: readonly Box[],
  ranks: ReadonlyMap<string, number>,
  perRow: number,
  areaOf: (b: Box) => string,
): Box[][] => {
  const maxRank = Math.max(0, ...ranks.values())
  const rows: Box[][] = []
  for (let r = maxRank; r >= 0; r--) {
    const inRank = boxes
      .filter((b) => ranks.get(b.key) === r)
      .sort((a, b) => (areaOf(a) < areaOf(b) ? -1 : areaOf(a) > areaOf(b) ? 1 : byName(a, b)))
    if (!inRank.length) continue
    const count = Math.ceil(inRank.length / perRow)
    const size = Math.ceil(inRank.length / count)
    for (let i = 0; i < inRank.length; i += size) rows.push(inRank.slice(i, i + size))
  }
  return rows
}

const portKey = (id: string): string => `port:${id}`
const dummyKey = (wire: string, row: number): string => `${wire}@${row}`

const linkWire = (link: BoxLink): Wire => ({ key: link.key, from: link.from, to: link.to, weight: link.breaks ? 2 : 1, link })
const flowWire = (flow: PortFlow): Wire => ({
  key: `${flow.box}⇢${flow.port}`,
  from: flow.box,
  to: portKey(flow.port),
  weight: 0.5,
  flow,
})

const innerRows = (a: number, b: number): number[] => {
  const out: number[] = []
  if (a < b) for (let k = a + 1; k < b; k++) out.push(k)
  else for (let k = a - 1; k > b; k--) out.push(k)
  return out
}

const sideKey = (key: string, isEnd: boolean): string => `${key}|${isEnd ? 'end' : 'start'}`

const endSides = (ra: number, rb: number): { from: boolean; to: boolean } => ({ from: ra <= rb, to: ra >= rb })

const fit = (d: readonly number[], seps: readonly number[], lo = -Infinity, hi = Infinity): number[] => {
  const n = d.length
  const before = [0]
  for (let i = 1; i < n; i++) before.push(before[i - 1] + seps[i - 1])
  const total = before[n - 1] ?? 0
  const clamped = d.map((x, i) => Math.min(hi - (total - before[i]), Math.max(lo + before[i], x)))
  const fwd = [...clamped]
  for (let i = 1; i < n; i++) fwd[i] = Math.max(fwd[i], fwd[i - 1] + seps[i - 1])
  const bwd = [...clamped]
  for (let i = n - 2; i >= 0; i--) bwd[i] = Math.min(bwd[i], bwd[i + 1] - seps[i])
  return fwd.map((x, i) => (x + bwd[i]) / 2)
}

const spread = (d: readonly number[], sep: number, lo: number, hi: number): number[] => {
  if (hi < lo) return d.map(() => (lo + hi) / 2)
  const step = d.length > 1 ? Math.min(sep, (hi - lo) / (d.length - 1)) : sep
  return fit(d, d.slice(1).map(() => step), lo, hi)
}

const sepOf = (a: Item, b: Item): number =>
  a.size / 2 + a.extra + b.size / 2 + (a.isReal && b.isReal ? a.gap : a.isReal || b.isReal ? GAP.dummy : GAP.dummies)

const packed = (row: readonly Item[]): number[] => {
  const at = [0]
  for (let i = 1; i < row.length; i++) at.push(at[i - 1] + sepOf(row[i - 1], row[i]))
  const mid = (at[at.length - 1] ?? 0) / 2
  return at.map((x) => x - mid)
}

const median = (values: readonly { at: number; w: number }[]): number => {
  const sorted = [...values].sort((a, b) => a.at - b.at)
  const half = sorted.reduce((s, v) => s + v.w, 0) / 2
  let acc = 0
  for (const v of sorted) {
    acc += v.w
    if (acc >= half) return v.at
  }
  return sorted[sorted.length - 1].at
}

interface Edge {
  u: string
  l: string
  w: number
}

const crossingsAt = (order: readonly Item[][], edges: readonly Edge[][], k: number): number => {
  const list = edges[k]
  if (!list?.length) return 0
  const up = new Map(order[k].map((it, i) => [it.key, i]))
  const low = new Map(order[k + 1].map((it, i) => [it.key, i]))
  const at = list.map((e) => ({ u: up.get(e.u) ?? 0, l: low.get(e.l) ?? 0, w: e.w }))
  let total = 0
  for (let i = 0; i < at.length; i++)
    for (let j = i + 1; j < at.length; j++) {
      if ((at[i].u - at[j].u) * (at[i].l - at[j].l) < 0) total += at[i].w * at[j].w
    }
  return total
}

const crossings = (order: readonly Item[][], edges: readonly Edge[][]): number =>
  edges.reduce((sum, _, k) => sum + crossingsAt(order, edges, k), 0)

const canSwap = (a: Item, b: Item): boolean => !a.isReal || !b.isReal || a.area === b.area

const transpose = (order: Item[][], edges: readonly Edge[][]): void => {
  const around = (k: number) => crossingsAt(order, edges, k - 1) + crossingsAt(order, edges, k)
  for (let pass = 0, isBetter = true; isBetter && pass < TRANSPOSE_PASSES; pass++) {
    isBetter = false
    order.forEach((row, k) => {
      for (let i = 0; i + 1 < row.length; i++) {
        if (!canSwap(row[i], row[i + 1])) continue
        const before = around(k)
        ;[row[i], row[i + 1]] = [row[i + 1], row[i]]
        if (around(k) < before) isBetter = true
        else [row[i], row[i + 1]] = [row[i + 1], row[i]]
      }
    })
  }
}

const sweep = (order: Item[][], edges: readonly Edge[][], isDown: boolean): void => {
  const n = order.length
  for (let s = 1; s < n; s++) {
    const k = isDown ? s : n - 1 - s
    const ref = isDown ? k - 1 : k + 1
    const refPacked = packed(order[ref])
    const refAt = new Map(order[ref].map((it, i) => [it.key, refPacked[i]]))
    const curAt = packed(order[k])
    const sums = new Map<string, { sum: number; w: number }>()
    for (const e of edges[isDown ? k - 1 : k]) {
      const self = isDown ? e.l : e.u
      const other = refAt.get(isDown ? e.u : e.l) ?? 0
      const acc = sums.get(self) ?? { sum: 0, w: 0 }
      acc.sum += other * e.w
      acc.w += e.w
      sums.set(self, acc)
    }
    const bary = order[k].map((it, i) => {
      const acc = sums.get(it.key)
      return { it, at: acc ? acc.sum / acc.w : curAt[i], i }
    })
    const areaAt = new Map<string, number[]>()
    for (const b of bary) if (b.it.isReal) areaAt.set(b.it.area, [...(areaAt.get(b.it.area) ?? []), b.at])
    const keyOf = (b: (typeof bary)[number]) => {
      const list = b.it.isReal ? areaAt.get(b.it.area)! : [b.at]
      return list.reduce((s, x) => s + x, 0) / list.length
    }
    order[k] = bary
      .sort((a, b) => keyOf(a) - keyOf(b) || a.at - b.at || a.i - b.i)
      .map((x) => x.it)
  }
}

const orderRows = (rows: Item[][], edges: readonly Edge[][]): Item[][] => {
  let best = rows.map((r) => [...r])
  let bestCount = crossings(best, edges)
  const cur = rows.map((r) => [...r])
  for (let i = 0; i < SWEEPS; i++) {
    sweep(cur, edges, i % 2 === 0)
    transpose(cur, edges)
    const count = crossings(cur, edges)
    if (count < bestCount) {
      bestCount = count
      best = cur.map((r) => [...r])
    }
  }
  return best
}

const centreOf = (order: readonly Item[][], skip: number): number => {
  const os = order.flatMap((row, k) => (k === skip ? [] : row.map((it) => it.o)))
  return os.length ? (Math.min(...os) + Math.max(...os)) / 2 : 0
}

const compact = (pulled: readonly (number | null)[], seps: readonly number[], centre: number): number[] => {
  const n = pulled.length
  const at = [0]
  for (let i = 1; i < n; i++) at.push(at[i - 1] + seps[i - 1])
  const known = pulled.flatMap((p, i) => (p === null ? [] : [i]))
  if (!known.length) return at.map((x) => x - at[n - 1] / 2 + centre)
  return pulled.map((p, i) => {
    if (p !== null) return p
    const left = [...known].reverse().find((j) => j < i)
    const right = known.find((j) => j > i)
    if (left !== undefined && right !== undefined) {
      const t = (at[i] - at[left]) / (at[right] - at[left])
      return pulled[left]! + (pulled[right]! - pulled[left]!) * t
    }
    return left !== undefined ? pulled[left]! + at[i] - at[left] : pulled[right!]! - (at[right!] - at[i])
  })
}

const placeAlongRows = (order: Item[][], edges: readonly Edge[][], chains: readonly string[][]): void => {
  for (const row of order) packed(row).forEach((o, i) => (row[i].o = o))
  const items = new Map(order.flat().map((it) => [it.key, it]))
  const neighbours = new Map<string, { item: Item; w: number }[]>()
  const add = (a: string, b: string, w: number) => {
    const other = items.get(b)
    if (other) neighbours.set(a, [...(neighbours.get(a) ?? []), { item: other, w }])
  }
  for (const list of edges)
    for (const e of list) {
      add(e.u, e.l, e.w)
      add(e.l, e.u, e.w)
    }
  const chainOf = new Map<string, Item[]>()
  for (const chain of chains) {
    const members = chain.flatMap((k) => items.get(k) ?? [])
    for (const m of members) chainOf.set(m.key, members)
  }
  for (let pass = 0; pass < PLACE_PASSES; pass++) {
    const ks = order.map((_, k) => k)
    if (pass % 2) ks.reverse()
    for (const k of ks) {
      const row = order[k]
      const pulled = row.map((it): number | null => {
        const chain = chainOf.get(it.key)
        if (chain && pass > 0) return chain.reduce((s, m) => s + m.o, 0) / chain.length
        const ns = neighbours.get(it.key)
        return ns?.length ? median(ns.map((n) => ({ at: n.item.o, w: n.w }))) : null
      })
      const seps = row.slice(1).map((it, i) => sepOf(row[i], it))
      fit(compact(pulled, seps, centreOf(order, k)), seps).forEach((o, i) => (row[i].o = o))
    }
  }
}

const gapsOf = (row: readonly Item[]): Gap[] => {
  const real = row.filter((it) => it.isReal).sort((a, b) => a.o - b.o)
  if (!real.length) return [{ lo: -Infinity, hi: Infinity }]
  const gaps: Gap[] = [{ lo: -Infinity, hi: real[0].o - real[0].size / 2 - GAP.lanePad }]
  for (let i = 1; i < real.length; i++) {
    gaps.push({
      lo: real[i - 1].o + real[i - 1].size / 2 + real[i - 1].extra + GAP.lanePad,
      hi: real[i].o - real[i].size / 2 - GAP.lanePad,
    })
  }
  const last = real[real.length - 1]
  gaps.push({ lo: last.o + last.size / 2 + last.extra + GAP.lanePad, hi: Infinity })
  return gaps
}

const intersect = (a: readonly Gap[], b: readonly Gap[]): Gap[] =>
  a.flatMap((x) => b.flatMap((y) => (Math.max(x.lo, y.lo) <= Math.min(x.hi, y.hi) ? [{ lo: Math.max(x.lo, y.lo), hi: Math.min(x.hi, y.hi) }] : [])))

const rangeOf = (it: Item): Gap => ({ lo: it.o - it.size / 2 + SIDE_PAD, hi: it.o + it.size / 2 - SIDE_PAD })

const channelsOf = (ra: number, rb: number): number[] => {
  if (ra === rb) return [ra]
  const out: number[] = []
  if (ra < rb) for (let c = ra; c < rb; c++) out.push(c)
  else for (let c = ra - 1; c >= rb; c--) out.push(c)
  return out
}

interface Segment {
  id: string
  channel: number
  top: number
  bot: number
  lo: number
  hi: number
  kind: number
}

const KIND = { u: 0, bundle: 1, right: 2, left: 3 } as const

const assignTracks = (segments: readonly Segment[]): { tracks: Map<string, number>; count: number } => {
  const bent = segments.filter((s) => s.kind !== KIND.right || Math.abs(s.top - s.bot) >= STRAIGHT)
  const width = (s: Segment) => s.hi - s.lo
  const sorted = [...bent].sort(
    (a, b) =>
      a.kind - b.kind ||
      (a.kind === KIND.u || a.kind === KIND.bundle ? width(a) - width(b) : a.kind === KIND.right ? b.top - a.top : a.top - b.top) ||
      (a.id < b.id ? -1 : 1),
  )
  const placed: { lo: number; hi: number; t: number; kind: number }[] = []
  const tracks = new Map<string, number>()
  for (const s of sorted) {
    const overlapping = placed.filter((p) => p.lo < s.hi + GAP.minTrack && p.hi > s.lo - GAP.minTrack)
    let t = Math.max(0, ...overlapping.filter((p) => p.kind === s.kind).map((p) => p.t + 1))
    while (overlapping.some((p) => p.t === t)) t++
    placed.push({ lo: s.lo, hi: s.hi, t, kind: s.kind })
    tracks.set(`${s.id}#${s.channel}`, t)
  }
  return { tracks, count: Math.max(0, ...placed.map((p) => p.t + 1)) }
}

interface Occupied {
  o: number
  r0: number
  r1: number
}

const placeLane = (free: readonly Gap[], ideal: number, r0: number, r1: number, taken: readonly Occupied[]): number | null => {
  const near = taken.filter((t) => t.r0 < r1 && t.r1 > r0)
  const candidates = [ideal, ...near.flatMap((t) => [t.o - GAP.lane, t.o + GAP.lane])]
  let best: number | null = null
  for (const g of free)
    for (const c of candidates) {
      const o = Math.min(g.hi, Math.max(g.lo, c))
      if (near.some((t) => Math.abs(t.o - o) < GAP.lane - 0.01)) continue
      if (best === null || Math.abs(o - ideal) < Math.abs(best - ideal)) best = o
    }
  return best
}

const isAdjacentInRow = (frame: Frame, a: Item, b: Item): boolean => {
  const [lo, hi] = a.o < b.o ? [a, b] : [b, a]
  return lo.extra === 0 && !frame.rows[a.row].items.some((it) => it.o > lo.o && it.o < hi.o)
}

const subtract = (gaps: readonly Gap[], cuts: readonly Gap[]): Gap[] =>
  cuts.reduce<Gap[]>(
    (acc, cut) =>
      acc.flatMap((g) =>
        cut.hi <= g.lo || cut.lo >= g.hi
          ? [g]
          : [
              ...(cut.lo > g.lo ? [{ lo: g.lo, hi: cut.lo }] : []),
              ...(cut.hi < g.hi ? [{ lo: cut.hi, hi: g.hi }] : []),
            ],
      ),
    [...gaps],
  )

const planWires = (frame: Frame, wires: readonly Wire[]): Plan => {
  const first = planOnce(frame, wires, new Map())
  const avoid = new Map<string, Gap[]>()
  for (const w of wires) {
    const a = frame.items.get(w.from)
    const b = frame.items.get(w.to)
    if (!a || !b || Math.abs(a.row - b.row) < 2) continue
    const lane = first.passages.get(dummyKey(w.key, innerRows(a.row, b.row)[0]))!
    const own = new Set([first.fanOut.get(w.key), first.fanIn.get(w.key)].map((x) => `bundle:${x}`))
    const chans = new Set(channelsOf(a.row, b.row))
    const cuts = first.bundles
      .filter((seg) => chans.has(seg.channel) && !own.has(seg.id) && lane > seg.lo && lane < seg.hi)
      .map((seg) => ({ lo: seg.lo - GAP.lane, hi: seg.hi + GAP.lane }))
    if (cuts.length) avoid.set(w.key, cuts)
  }
  if (!avoid.size) return first
  const second = planOnce(frame, wires, avoid)
  const live = wires.filter((w) => frame.items.has(w.from) && frame.items.has(w.to))
  const cost = (plan: Plan) => routeCost(live.map((w) => drawWire(frame, plan, w)))
  return cost(second) < cost(first) ? second : first
}

const planOnce = (frame: Frame, wires: readonly Wire[], avoid: ReadonlyMap<string, readonly Gap[]>): Plan => {
  const rowSpan = (k: number): Span => {
    const items = frame.rows[k].items
    const start = Math.min(...items.map((it) => it.r))
    return { start, end: Math.max(...items.map((it) => it.r + it.depth)) }
  }
  const spans = frame.rows.map((_, k) => rowSpan(k))
  const passages = new Map<string, number>()
  const taken: Occupied[] = []
  const live = wires.filter((w) => frame.items.has(w.from) && frame.items.has(w.to))
  for (const w of live) {
    const from = frame.items.get(w.from)!
    const to = frame.items.get(w.to)!
    const inner = innerRows(from.row, to.row)
    if (!inner.length) continue
    const dummies = inner.flatMap((k) => frame.dummies.get(dummyKey(w.key, k)) ?? [])
    const ideal = dummies.length ? dummies.reduce((s, x) => s + x, 0) / dummies.length : (from.o + to.o) / 2
    const lo = Math.min(...inner.map((k) => spans[k].start))
    const hi = Math.max(...inner.map((k) => spans[k].end))
    const free = subtract(
      inner.reduce<Gap[]>((acc, k) => intersect(acc, frame.rows[k].gaps), [{ lo: -Infinity, hi: Infinity }]),
      avoid.get(w.key) ?? [],
    )
    const tries = [intersect(intersect(free, [rangeOf(from)]), [rangeOf(to)]), intersect(free, [rangeOf(from)]), intersect(free, [rangeOf(to)]), free]
    let lane: number | null = null
    for (const t of tries) {
      if (lane === null && t.length) lane = placeLane(t, ideal, lo, hi, taken)
    }
    if (lane !== null) {
      taken.push({ o: lane, r0: lo, r1: hi })
      for (const k of inner) passages.set(dummyKey(w.key, k), lane)
      continue
    }
    inner.forEach((k, i) => {
      const t = (i + 1) / (inner.length + 1)
      const want = dummies[i] ?? from.o + (to.o - from.o) * t
      const o = placeLane(frame.rows[k].gaps, want, spans[k].start, spans[k].end, taken) ?? want
      taken.push({ o, r0: spans[k].start, r1: spans[k].end })
      passages.set(dummyKey(w.key, k), o)
    })
  }

  const sideOf = (w: Wire, atFrom: boolean): string => {
    const a = frame.items.get(w.from)!
    const b = frame.items.get(w.to)!
    const at = endSides(a.row, b.row)
    return sideKey(atFrom ? w.from : w.to, atFrom ? at.from : at.to)
  }
  const continuation = (w: Wire, atFrom: boolean): number => {
    const a = frame.items.get(w.from)!
    const b = frame.items.get(w.to)!
    const inner = innerRows(a.row, b.row)
    if (inner.length) return passages.get(dummyKey(w.key, inner[atFrom ? 0 : inner.length - 1]))!
    const own = rangeOf(atFrom ? a : b)
    const other = rangeOf(atFrom ? b : a)
    const lo = Math.max(own.lo, other.lo)
    const hi = Math.min(own.hi, other.hi)
    if (lo <= hi && a.row !== b.row) return (lo + hi) / 2
    return (atFrom ? b : a).o
  }
  const isStraightEnd = (w: Wire, atFrom: boolean): boolean => {
    const own = rangeOf(frame.items.get(atFrom ? w.from : w.to)!)
    const c = continuation(w, atFrom)
    return c >= own.lo && c <= own.hi
  }
  const isSameRow = (w: Wire) => frame.items.get(w.from)!.row === frame.items.get(w.to)!.row
  const groups = (atFrom: boolean) => {
    const out = new Map<string, Wire[]>()
    for (const w of live) {
      if (isSameRow(w) || isStraightEnd(w, atFrom)) continue
      const s = sideOf(w, atFrom)
      out.set(s, [...(out.get(s) ?? []), w])
    }
    return new Map([...out].filter(([, list]) => list.length >= BUNDLE_MIN))
  }
  const outs = groups(true)
  const ins = groups(false)
  const fanOut = new Map<string, string>()
  const fanIn = new Map<string, string>()
  for (const [s, list] of outs) for (const w of list) fanOut.set(w.key, s)
  for (const [s, list] of ins) for (const w of list) fanIn.set(w.key, s)
  for (const w of live) {
    const a = frame.items.get(w.from)!
    const b = frame.items.get(w.to)!
    if (Math.abs(a.row - b.row) !== 1 || !fanOut.has(w.key) || !fanIn.has(w.key)) continue
    const outSize = outs.get(fanOut.get(w.key)!)!.length
    const inSize = ins.get(fanIn.get(w.key)!)!.length
    if (outSize > inSize) fanIn.delete(w.key)
    else fanOut.delete(w.key)
  }
  const trunkAt = new Map<string, number[]>()
  for (const w of live) {
    if (isSameRow(w)) continue
    for (const atFrom of [true, false]) {
      const s = sideOf(w, atFrom)
      const own = atFrom ? fanOut : fanIn
      const other = atFrom ? fanIn : fanOut
      if (!(atFrom ? outs : ins).has(s) || own.has(w.key) || other.has(w.key) || !isStraightEnd(w, atFrom)) continue
      own.set(w.key, s)
      trunkAt.set(s, [...(trunkAt.get(s) ?? []), continuation(w, atFrom)])
    }
  }

  const ports = new Map<string, number>()
  const sides = new Map<string, { id: string; desired: number }[]>()
  const pushPort = (s: string, id: string, desired: number) => {
    const list = sides.get(s) ?? []
    const hit = list.find((x) => x.id === id)
    if (hit) hit.desired = (hit.desired + desired) / 2
    else list.push({ id, desired })
    sides.set(s, list)
  }
  for (const w of live) {
    for (const atFrom of [true, false]) {
      const bundle = (atFrom ? fanOut : fanIn).get(w.key)
      const straight = bundle ? trunkAt.get(bundle) : undefined
      const desired = straight ? straight.reduce((s, x) => s + x, 0) / straight.length : continuation(w, atFrom)
      pushPort(sideOf(w, atFrom), bundle ? `trunk:${bundle}` : `${w.key}:${atFrom ? 's' : 't'}`, desired)
    }
  }
  const step = AXIS[frame.orient].portStep
  for (const [s, list] of sides) {
    const item = frame.items.get(s.slice(0, s.lastIndexOf('|')))!
    const range = rangeOf(item)
    list.sort((a, b) => a.desired - b.desired || (a.id < b.id ? -1 : 1))
    spread(
      list.map((x) => Math.min(range.hi, Math.max(range.lo, x.desired))),
      step,
      range.lo,
      range.hi,
    ).forEach((o, i) => ports.set(list[i].id, o))
  }
  const portOf = (w: Wire, atFrom: boolean): number => {
    const bundle = (atFrom ? fanOut : fanIn).get(w.key)
    return ports.get(bundle ? `trunk:${bundle}` : `${w.key}:${atFrom ? 's' : 't'}`)!
  }
  for (const w of live) {
    if (fanOut.has(w.key) || fanIn.has(w.key) || isSameRow(w)) continue
    const p = portOf(w, true)
    const q = portOf(w, false)
    if (Math.abs(p - q) > SNAP || Math.abs(p - q) < 0.01) continue
    const target = rangeOf(frame.items.get(w.to)!)
    if (p >= target.lo && p <= target.hi) ports.set(`${w.key}:t`, p)
  }

  const segments: Segment[] = []
  const seqOf = (w: Wire): number[] => {
    const a = frame.items.get(w.from)!
    const b = frame.items.get(w.to)!
    return [portOf(w, true), ...innerRows(a.row, b.row).map((k) => passages.get(dummyKey(w.key, k))!), portOf(w, false)]
  }
  for (const w of live) {
    const a = frame.items.get(w.from)!
    const b = frame.items.get(w.to)!
    const seq = seqOf(w)
    if (a.row === b.row) {
      if (isAdjacentInRow(frame, a, b)) continue
      segments.push({ id: w.key, channel: a.row, top: seq[0], bot: seq[1], lo: Math.min(seq[0], seq[1]), hi: Math.max(seq[0], seq[1]), kind: KIND.u })
      continue
    }
    const chans = channelsOf(a.row, b.row)
    const isDown = a.row < b.row
    chans.forEach((c, i) => {
      const top = isDown ? seq[i] : seq[i + 1]
      const bot = isDown ? seq[i + 1] : seq[i]
      const bundle = i === 0 ? fanOut.get(w.key) : i === chans.length - 1 ? fanIn.get(w.key) : undefined
      segments.push({
        id: bundle ? `bundle:${bundle}` : w.key,
        channel: c,
        top,
        bot,
        lo: Math.min(top, bot),
        hi: Math.max(top, bot),
        kind: bundle ? KIND.bundle : top < bot ? KIND.right : KIND.left,
      })
    })
  }
  const merged = new Map<string, Segment>()
  for (const s of segments) {
    const k = `${s.id}#${s.channel}`
    const hit = merged.get(k)
    if (hit) {
      hit.lo = Math.min(hit.lo, s.lo)
      hit.hi = Math.max(hit.hi, s.hi)
    } else merged.set(k, { ...s })
  }
  const all = [...merged.values()]
  const tracks = new Map<string, number>()
  const counts = frame.rows.map((_, c) => {
    const result = assignTracks(all.filter((s) => s.channel === c))
    for (const [k, t] of result.tracks) tracks.set(k, t)
    return result.count
  })
  for (const w of live) {
    for (const c of channelsOf(frame.items.get(w.from)!.row, frame.items.get(w.to)!.row)) {
      const bundle = [fanOut.get(w.key), fanIn.get(w.key)].find((b) => b && tracks.has(`bundle:${b}#${c}`))
      if (bundle) tracks.set(`${w.key}#${c}`, tracks.get(`bundle:${bundle}#${c}`)!)
    }
  }
  for (const w of live) {
    for (const atFrom of [true, false]) {
      const bundle = (atFrom ? fanOut : fanIn).get(w.key)
      if (!bundle) ports.set(`${w.key}:${atFrom ? 's' : 't'}`, portOf(w, atFrom))
      else ports.set(`${w.key}:${atFrom ? 's' : 't'}`, ports.get(`trunk:${bundle}`)!)
    }
  }
  return { ports, passages, tracks, counts, fanOut, fanIn, bundles: all.filter((seg) => seg.kind === KIND.bundle) }
}

const toXY = (orient: Orient, o: number, r: number): Point => (orient === 'rows' ? { x: o, y: r } : { x: r, y: o })

const rectOf = (orient: Orient, o0: number, r0: number, oSize: number, rSize: number): Rect => {
  const p = toXY(orient, o0, r0)
  return orient === 'rows' ? { ...p, w: oSize, h: rSize } : { ...p, w: rSize, h: oSize }
}

const itemRect = (orient: Orient, it: Item): Rect => rectOf(orient, it.o - it.size / 2, it.r, it.size, it.depth)

const trackR = (frame: Frame, plan: Plan, wire: string, c: number): number => {
  const ch = frame.channels[c]
  const mid = (ch.start + ch.end) / 2
  const t = plan.tracks.get(`${wire}#${c}`)
  const n = plan.counts[c]
  if (t === undefined || n < 2) return mid
  const step = Math.max(GAP.minTrack, Math.min(GAP.track, (ch.end - ch.start - 2 * GAP.edge) / (n - 1)))
  return mid + (t - (n - 1) / 2) * step
}

interface Drawn {
  d: string
  label: Point
  points: readonly Point[]
}

const drawWire = (frame: Frame, plan: Plan, w: Wire): Drawn => {
  const from = frame.items.get(w.from)!
  const to = frame.items.get(w.to)!
  const axis = AXIS[frame.orient]
  const xy = (o: number, r: number) => toXY(frame.orient, o, r)
  if (from.row === to.row && isAdjacentInRow(frame, from, to)) {
    const r = from.r + from.depth / 2
    const isRight = from.o < to.o
    const start = from.o + (isRight ? from.size / 2 : -from.size / 2)
    const end = to.o + (isRight ? -to.size / 2 - 1 : to.size / 2 + 1)
    const path = roundedPath([xy(start, r), xy(end, r)], CORNER)
    return { ...path, label: alongRun(xy(start, r), xy(end, r), axis.label) }
  }
  const inner = innerRows(from.row, to.row)
  const seq = [plan.ports.get(`${w.key}:s`)!, ...inner.map((k) => plan.passages.get(dummyKey(w.key, k))!), plan.ports.get(`${w.key}:t`)!]
  const at = endSides(from.row, to.row)
  const startR = at.from ? from.r + from.depth : from.r
  const endR = at.to ? to.r + to.depth : to.r
  const pts: { o: number; r: number }[] = [{ o: seq[0], r: startR }]
  const chans = channelsOf(from.row, to.row)
  chans.forEach((c, i) => {
    const r = trackR(frame, plan, w.key, c)
    pts.push({ o: seq[i], r }, { o: seq[i + 1], r })
  })
  pts.push({ o: seq[seq.length - 1], r: endR + (at.to ? 1 : -1) })
  const dir = at.from ? 1 : -1
  const label = plan.fanOut.has(w.key)
    ? { o: seq[1], r: trackR(frame, plan, w.key, chans[0]) + dir * axis.label }
    : { o: seq[0], r: startR + dir * axis.label }
  const path = roundedPath(
    pts.map((p) => xy(p.o, p.r)),
    CORNER,
  )
  return { ...path, label: xy(label.o, label.r) }
}

const segmentsCross = (a: Point, b: Point, c: Point, d: Point): boolean => {
  const den = (b.x - a.x) * (d.y - c.y) - (b.y - a.y) * (d.x - c.x)
  if (Math.abs(den) < 1e-9) return false
  const t = ((c.x - a.x) * (d.y - c.y) - (c.y - a.y) * (d.x - c.x)) / den
  const u = ((c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)) / den
  return t > 0.001 && t < 0.999 && u > 0.001 && u < 0.999
}

const crossCount = (drawn: readonly Drawn[]): number => {
  let cross = 0
  for (let i = 0; i < drawn.length; i++)
    for (let j = i + 1; j < drawn.length; j++) {
      const [p, q] = [drawn[i].points, drawn[j].points]
      for (let s = 1; s < p.length; s++)
        for (let k = 1; k < q.length; k++) if (segmentsCross(p[s - 1], p[s], q[k - 1], q[k])) cross++
    }
  return cross
}

const DETOUR_LENGTH = 0.03

const lengthOf = (drawn: readonly Drawn[]): number =>
  drawn.reduce((n, d) => n + d.points.slice(1).reduce((s, p, i) => s + Math.abs(p.x - d.points[i].x) + Math.abs(p.y - d.points[i].y), 0), 0)

const routeCost = (drawn: readonly Drawn[]): number =>
  SCORE.crossing * crossCount(drawn) + SCORE.corner * drawn.reduce((n, d) => n + Math.max(0, d.points.length - 2), 0) + DETOUR_LENGTH * lengthOf(drawn)

const scoreOf = (drawn: readonly Drawn[], frame: Frame): number => {
  const rows = frame.rows
  const cross = crossCount(drawn)
  const corners = drawn.reduce((n, d) => n + Math.max(0, d.points.length - 2), 0)
  const length = lengthOf(drawn)
  const widths = rows.map((r) => {
    const its = r.items
    return its.length ? Math.max(...its.map((it) => it.o + it.size / 2 + it.extra)) - Math.min(...its.map((it) => it.o - it.size / 2)) : 0
  })
  const mean = widths.reduce((s, x) => s + x, 0) / Math.max(1, widths.length)
  const imbalance = (Math.max(0, ...widths) - mean) / AXIS.rows.box.o
  const across = Math.max(0, ...widths)
  const along = frame.channels[frame.channels.length - 1]?.end ?? 1
  const aspect = frame.orient === 'rows' ? across / along : along / across
  const skew = Math.abs(Math.log2(aspect / (16 / 9)))
  return SCORE.crossing * cross + SCORE.corner * corners + SCORE.length * length + SCORE.imbalance * imbalance + SCORE.aspect * skew
}

export const shownLinks = (
  links: readonly BoxLink[],
  selected: string | null,
  stubbed: ReadonlySet<string> = new Set(),
): readonly BoxLink[] => {
  if (selected !== null) return links.filter((l) => l.from === selected || l.to === selected)
  const real = links.filter((l) => !stubbed.has(l.key))
  const top = new Set(real.slice(0, TOP_LINKS).map((l) => l.key))
  return real.filter((l) => top.has(l.key) || l.breaks)
}

export const isDrawnUnselected = (links: readonly BoxLink[], link: BoxLink): boolean =>
  link.breaks || links.indexOf(link) < TOP_LINKS

export const shownStubs = (stubs: readonly PlacedStub[], selected: string | null): readonly PlacedStub[] =>
  selected === null ? stubs : stubs.filter((s) => s.link.from !== selected && s.link.to !== selected)

const hubsOf = (boxes: readonly Box[], links: readonly BoxLink[], ranks: ReadonlyMap<string, number>): Set<string> => {
  const sources = new Map<string, Set<string>>()
  for (const l of links) sources.set(l.to, (sources.get(l.to) ?? new Set()).add(l.from))
  const min = Math.max(HUB.minIn, HUB.share * boxes.length)
  const hubs = new Set<string>()
  for (const [to, from] of sources) {
    if (from.size < min) continue
    const below = [...from].filter((f) => (ranks.get(f) ?? 0) - (ranks.get(to) ?? 0) >= HUB.ranksBelow)
    if (below.length) hubs.add(to)
  }
  return hubs
}

const chipWidth = (text: string): number => Math.min(CHIP.maxW, Math.max(CHIP.minW, text.length * CHIP.charPx + CHIP.pad))

interface Built {
  layout: BoxesLayout
  score: number
}

const build = (
  data: Boxes,
  ranks: ReadonlyMap<string, number>,
  ports: readonly PortTarget[],
  flows: readonly PortFlow[],
  orient: Orient,
  perRow: number,
  areaOf: (b: Box) => string,
  stubs: ReadonlyMap<string, BoxLink[]>,
  stubbed: ReadonlySet<string>,
): Built => {
  const axis = AXIS[orient]
  const names = new Map(data.boxes.map((b) => [b.key, b.name]))
  const boxRows = rowsOf(data.boxes, ranks, perRow, areaOf)
  const rowOf = new Map<string, number>()
  boxRows.forEach((row, k) => row.forEach((b) => rowOf.set(b.key, k)))
  const portRow = boxRows.length
  for (const p of ports) rowOf.set(portKey(p.id), portRow)
  const known = (w: Wire) => rowOf.has(w.from) && rowOf.has(w.to)
  const flowWires = flows.map(flowWire).filter(known)
  const linkWires = (links: readonly BoxLink[]) => links.map(linkWire).filter(known)

  const ends = new Map<string, number>()
  for (const w of [...linkWires(data.links.filter((l) => !stubbed.has(l.key))), ...flowWires]) {
    const at = endSides(rowOf.get(w.from)!, rowOf.get(w.to)!)
    for (const s of [sideKey(w.from, at.from), sideKey(w.to, at.to)]) ends.set(s, (ends.get(s) ?? 0) + 1)
  }
  const sizeOf = (key: string, base: number): number => {
    const n = Math.max(ends.get(sideKey(key, true)) ?? 0, ends.get(sideKey(key, false)) ?? 0)
    return Math.min(base * SIDE_GROWTH, Math.max(base, n * axis.portStep + 2 * SIDE_PAD))
  }
  const chipText = (l: BoxLink) => `${names.get(l.to) ?? l.to} ${l.count}`
  const extraOf = (key: string): number => {
    const own = stubs.get(key)
    if (!own?.length) return 0
    return CHIP.gap + (orient === 'rows' ? Math.max(...own.map((l) => chipWidth(chipText(l)))) : CHIP.h)
  }
  const item = (key: string, row: number, size: number, depth: number, isReal: boolean, area: string): Item => ({
    key,
    row,
    o: 0,
    r: 0,
    size,
    extra: isReal ? extraOf(key) : 0,
    depth,
    isReal,
    gap: axis.boxGap,
    area,
  })
  const rows: Item[][] = boxRows.map((row, k) =>
    row.map((b) => item(b.key, k, sizeOf(b.key, axis.box.o), axis.box.r, true, areaOf(b))),
  )
  if (ports.length) {
    rows.push(ports.map((p) => item(portKey(p.id), portRow, sizeOf(portKey(p.id), axis.port.o), axis.port.r, true, 'port')))
  }

  const edges: Edge[][] = rows.slice(1).map(() => [])
  const dummyKeys = new Set<string>()
  const chains: string[][] = []
  for (const w of [...linkWires(shownLinks(data.links, null, stubbed)), ...flowWires]) {
    const [ra, rb] = [rowOf.get(w.from)!, rowOf.get(w.to)!]
    if (ra === rb) continue
    const inner = innerRows(ra, rb)
    const chain = [w.from, ...inner.map((k) => dummyKey(w.key, k)), w.to]
    const ks = [ra, ...inner, rb]
    inner.forEach((k, i) => {
      rows[k].push(item(chain[i + 1], k, 0, 0, false, ''))
      dummyKeys.add(chain[i + 1])
    })
    if (inner.length) chains.push(chain.slice(1, -1))
    if (ra > rb) {
      chain.reverse()
      ks.reverse()
    }
    for (let i = 1; i < chain.length; i++) edges[ks[i - 1]].push({ u: chain[i - 1], l: chain[i], w: w.weight })
  }
  const ordered = orderRows(rows, edges)
  placeAlongRows(ordered, edges, chains)
  const all = ordered.flat()
  const real = all.filter((it) => it.isReal)
  const left = all.length ? Math.min(...all.map((it) => it.o - it.size / 2)) : 0
  for (const it of all) it.o -= left

  const frame: Frame = {
    orient,
    rows: ordered.map((row) => ({ items: row.filter((it) => it.isReal).sort((a, b) => a.o - b.o), gaps: gapsOf(row) })),
    channels: ordered.map(() => ({ start: 0, end: 0 })),
    items: new Map(real.map((it) => [it.key, it])),
    dummies: new Map(all.flatMap((it) => (dummyKeys.has(it.key) ? [[it.key, it.o] as const] : []))),
  }
  let r = 0
  frame.rows.forEach((row) => {
    const depth = Math.max(0, ...row.items.map((it) => it.depth))
    for (const it of row.items) it.r = r + (depth - it.depth) / 2
    r += depth + axis.channel
  })
  const defaults = [...linkWires(shownLinks(data.links, null, stubbed)), ...flowWires]
  const counts = planOnce(frame, defaults, new Map()).counts
  r = 0
  frame.rows.forEach((row, k) => {
    const depth = Math.max(0, ...row.items.map((it) => it.depth))
    for (const it of row.items) it.r = r + (depth - it.depth) / 2
    r += depth
    const tracks = Math.max(0, counts[k] - 1) * GAP.track + 2 * GAP.edge
    const isLast = k === frame.rows.length - 1
    const thick = isLast ? (counts[k] ? tracks : 0) : Math.max(axis.channel, tracks)
    frame.channels[k] = { start: r, end: r + thick }
    r += thick
  })
  const plan = planWires(frame, defaults)
  const score = scoreOf(
    defaults.map((w) => drawWire(frame, plan, w)),
    frame,
  )

  const boxByKey = new Map(data.boxes.map((b) => [b.key, b]))
  const usedBy = new Map<string, number>()
  for (const list of stubs.values()) for (const l of list) usedBy.set(l.to, (usedBy.get(l.to) ?? 0) + 1)
  const placed = real.flatMap((it): PlacedBox[] => {
    const box = boxByKey.get(it.key)
    return box ? [{ box, usedBy: usedBy.get(box.key) ?? 0, ...itemRect(orient, it) }] : []
  })
  const placedPorts = ports.flatMap((port): PlacedPort[] => {
    const it = frame.items.get(portKey(port.id))
    return it ? [{ port, ...itemRect(orient, it) }] : []
  })
  const placedStubs = real.flatMap((it): PlacedStub[] => {
    const own = stubs.get(it.key) ?? []
    if (!own.length) return []
    const widths = own.map((l) => chipWidth(chipText(l)))
    const along = orient === 'rows' ? own.map(() => CHIP.h) : widths.map((w) => Math.min(w, (it.depth - (own.length - 1) * CHIP.between) / own.length))
    const total = along.reduce((s, x) => s + x, 0) + (own.length - 1) * CHIP.between
    let rAt = it.r + (it.depth - total) / 2
    const o0 = it.o + it.size / 2 + CHIP.gap
    return own.map((link, i) => {
      const oSize = orient === 'rows' ? widths[i] : CHIP.h
      const rect = rectOf(orient, o0, rAt, oSize, along[i])
      const mid = rAt + along[i] / 2
      rAt += along[i] + CHIP.between
      const a = toXY(orient, it.o + it.size / 2, mid)
      const b = toXY(orient, o0, mid)
      return { link, hubName: names.get(link.to) ?? link.to, ...rect, d: `M${a.x},${a.y} L${b.x},${b.y}` }
    })
  })
  const areas = frame.rows.flatMap((row): AreaFrame[] => {
    const byArea = new Map<string, Item[]>()
    for (const it of row.items) if (it.area !== 'port') byArea.set(it.area, [...(byArea.get(it.area) ?? []), it])
    if (byArea.size < 2 || ![...byArea.values()].some((its) => its.length > 1)) return []
    return [...byArea].filter(([, its]) => its.length > 1).map(([name, its]) => {
      const lo = Math.min(...its.map((it) => it.o - it.size / 2)) - GAP.lanePad
      const hi = Math.max(...its.map((it) => it.o + it.size / 2 + it.extra)) + GAP.lanePad
      const r0 = Math.min(...its.map((it) => it.r)) - GAP.lanePad
      const r1 = Math.max(...its.map((it) => it.r + it.depth)) + GAP.lanePad
      return { name, ...rectOf(orient, lo, r0, hi - lo, r1 - r0) }
    })
  })
  const right = Math.max(0, ...all.map((it) => it.o + it.size / 2 + it.extra))
  const a = toXY(orient, -GAP.edge, -GAP.edge)
  const b = toXY(orient, right + GAP.edge, r)
  return {
    score,
    layout: {
      boxes: placed,
      byKey: new Map(placed.map((p) => [p.box.key, p])),
      links: data.links,
      stubbed,
      stubs: placedStubs,
      areas,
      ports: placedPorts,
      flows: flowWires.map((w) => w.flow!),
      bounds: { x: a.x - MARGIN, y: a.y - MARGIN, w: b.x - a.x + MARGIN * 2, h: b.y - a.y + MARGIN * 2 },
      frame,
    },
  }
}

export const layoutBoxes = (
  data: Boxes,
  layerRanks: ReadonlyMap<number, number>,
  ports: readonly PortTarget[],
  flows: readonly PortFlow[],
  orient: Orient = 'rows',
  areaOf: (b: Box) => string = (b) => b.kind,
): BoxesLayout => {
  const ranks = layerRanksOf(data.boxes, layerRanks) ?? ranksOf(data.boxes, data.links)
  const hubs = hubsOf(data.boxes, data.links, ranks)
  const stubs = new Map<string, BoxLink[]>()
  const stubbed = new Set<string>()
  for (const l of data.links) {
    if (!hubs.has(l.to) || l.breaks || (ranks.get(l.from) ?? 0) - (ranks.get(l.to) ?? 0) < HUB.ranksBelow) continue
    stubbed.add(l.key)
    stubs.set(l.from, [...(stubs.get(l.from) ?? []), l])
  }
  const perRank = new Map<number, number>()
  for (const r of ranks.values()) perRank.set(r, (perRank.get(r) ?? 0) + 1)
  const widest = Math.max(0, ...perRank.values())
  const tries = PER_ROW.filter((_, i) => i === 0 || widest > PER_ROW[i - 1])
  let best: Built | null = null
  for (const perRow of tries) {
    const built = build(data, ranks, ports, flows, orient, perRow, areaOf, stubs, stubbed)
    if (!best || built.score < best.score) best = built
  }
  return best!.layout
}

export const routeLinks = (
  layout: BoxesLayout,
  shown: readonly BoxLink[],
): { links: readonly PlacedLink[]; flows: readonly PlacedFlow[] } => {
  const frame = layout.frame
  const known = (w: Wire) => frame.items.has(w.from) && frame.items.has(w.to)
  const links = shown.map(linkWire).filter(known)
  const flows = layout.flows.map(flowWire).filter(known)
  const plan = planWires(frame, [...links, ...flows])
  const strip = ({ d, label }: Drawn) => ({ d, label })
  return {
    links: links.map((w) => ({ link: w.link!, ...strip(drawWire(frame, plan, w)) })),
    flows: flows.map((w) => ({ flow: w.flow!, ...strip(drawWire(frame, plan, w)) })),
  }
}
