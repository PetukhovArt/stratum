export interface Point {
  x: number
  y: number
}

export interface Rect extends Point {
  w: number
  h: number
}

export type RectSide = 'left' | 'right' | 'top' | 'bottom'

export interface GridLine {
  at: number
  cost: number
  room: number
}

interface RouteGrid {
  xs: readonly GridLine[]
  ys: readonly GridLine[]
  freeH: Uint8Array
  freeV: Uint8Array
  blocks: readonly Rect[]
}

export interface RouteLink {
  key: string
  from: Rect
  to: Rect
  weight?: number
  sides?: readonly RectSide[]
  bundle?: string
  joinAt?: 'from' | 'to'
}

export interface RouteOptions {
  obstacles: readonly Rect[]
  lines?: { xs: readonly GridLine[]; ys: readonly GridLine[] }
  bend?: number
  track?: number
  corner?: number
}

export interface Route {
  d: string
  points: readonly Point[]
  label: Point
}

const MERGE_PX = 2
const BEND = 26
const GUTTER_ROOM = 24
const SHARED = 0.35
const CROWDED = 0.7
const CROSS = 60
const PORT_KEY_SCALE = 1e-3
const TRACK_PX = 2.5
const CORNER_PX = 4
const MAX_EXPANDED = 400_000

const mergeLines = (lines: readonly GridLine[]): GridLine[] => {
  const sorted = [...lines].sort((a, b) => a.at - b.at)
  const out: GridLine[] = []
  for (const l of sorted) {
    const last = out[out.length - 1]
    if (last && l.at - last.at <= MERGE_PX) {
      if (l.cost < last.cost) out[out.length - 1] = { ...l, room: Math.min(l.room, last.room) }
      else last.room = Math.min(l.room, last.room)
    } else out.push({ ...l })
  }
  return out
}

const spans = (blocks: readonly Rect[], across: (r: Rect) => [number, number], along: (r: Rect) => [number, number]) =>
  (at: number): [number, number][] =>
    blocks.filter((r) => {
      const [lo, hi] = across(r)
      return at > lo && at < hi
    }).map(along)

const freeRuns = (lines: readonly GridLine[], blockers: [number, number][]): Uint8Array => {
  const free = new Uint8Array(Math.max(0, lines.length - 1))
  for (let i = 0; i + 1 < lines.length; i++) {
    const a = lines[i].at
    const b = lines[i + 1].at
    free[i] = blockers.some(([lo, hi]) => lo < b && hi > a) ? 0 : 1
  }
  return free
}

const gutterLines = (blocks: readonly Rect[]): { xs: GridLine[]; ys: GridLine[] } => {
  const along = (lo: (r: Rect) => number, size: (r: Rect) => number): GridLine[] => {
    const edges = [...new Set(blocks.flatMap((r) => [lo(r), lo(r) + size(r)]))].sort((a, b) => a - b)
    const lines: GridLine[] = []
    if (edges.length) {
      lines.push({ at: edges[0] - GUTTER_ROOM, cost: 1, room: GUTTER_ROOM })
      lines.push({ at: edges[edges.length - 1] + GUTTER_ROOM, cost: 1, room: GUTTER_ROOM })
    }
    for (let i = 1; i < edges.length; i++) {
      const gap = edges[i] - edges[i - 1]
      if (gap > 0) lines.push({ at: (edges[i] + edges[i - 1]) / 2, cost: 1, room: Math.max(0, Math.min(GUTTER_ROOM, gap / 2 - 2)) })
    }
    return lines
  }
  return { xs: along((r) => r.x, (r) => r.w), ys: along((r) => r.y, (r) => r.h) }
}

const routeGrid = (xLines: readonly GridLine[], yLines: readonly GridLine[], blocks: readonly Rect[]): RouteGrid => {
  const xs = mergeLines(xLines)
  const ys = mergeLines(yLines)
  const nx = xs.length
  const ny = ys.length
  const freeH = new Uint8Array(nx * ny)
  const freeV = new Uint8Array(nx * ny)
  const rowBlocks = spans(blocks, (r) => [r.y, r.y + r.h], (r) => [r.x, r.x + r.w])
  const colBlocks = spans(blocks, (r) => [r.x, r.x + r.w], (r) => [r.y, r.y + r.h])
  for (let j = 0; j < ny; j++) {
    const run = freeRuns(xs, rowBlocks(ys[j].at))
    for (let i = 0; i + 1 < nx; i++) freeH[j * nx + i] = run[i]
  }
  for (let i = 0; i < nx; i++) {
    const run = freeRuns(ys, colBlocks(xs[i].at))
    for (let j = 0; j + 1 < ny; j++) freeV[j * nx + i] = run[j]
  }
  return { xs, ys, freeH, freeV, blocks }
}

const CLEAR_PX = 1.5

const clearance = (g: RouteGrid, axis: number, at: number, lo: number, hi: number): number => {
  let room = Infinity
  for (const b of g.blocks) {
    const [s0, s1, t0, t1] = axis === 0 ? [b.x, b.x + b.w, b.y, b.y + b.h] : [b.y, b.y + b.h, b.x, b.x + b.w]
    if (s0 >= hi || s1 <= lo) continue
    if (t1 <= at) room = Math.min(room, at - t1)
    else if (t0 >= at) room = Math.min(room, t0 - at)
  }
  return room - CLEAR_PX
}

const DIRS = [
  [1, 0],
  [-1, 0],
  [0, 1],
  [0, -1],
] as const

class Heap {
  private keys: number[] = []
  private vals: number[] = []
  get top() {
    return this.keys[0]
  }
  get size() {
    return this.keys.length
  }
  push(key: number, val: number) {
    const k = this.keys
    const v = this.vals
    let i = k.length
    k.push(key)
    v.push(val)
    while (i > 0) {
      const p = (i - 1) >> 1
      if (k[p] <= key) break
      k[i] = k[p]
      v[i] = v[p]
      i = p
    }
    k[i] = key
    v[i] = val
  }
  pop(): number {
    const k = this.keys
    const v = this.vals
    const top = v[0]
    const lastK = k.pop()!
    const lastV = v.pop()!
    if (k.length) {
      let i = 0
      for (;;) {
        let c = 2 * i + 1
        if (c >= k.length) break
        if (c + 1 < k.length && k[c + 1] < k[c]) c++
        if (k[c] >= lastK) break
        k[i] = k[c]
        v[i] = v[c]
        i = c
      }
      k[i] = lastK
      v[i] = lastV
    }
    return top
  }
}

interface Side {
  ix: number
  iy: number
  out: number
  isTopOrBottom: boolean
}

interface Gate {
  node: number
  dir: number
  cost: number
  side: Side
}

const nearest = (lines: readonly GridLine[], below: number, above: number): [number, number] => {
  let lo = -1
  let hi = -1
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].at < below) lo = i
    if (hi < 0 && lines[i].at > above) hi = i
  }
  return [lo, hi]
}

const ALL_SIDES: readonly RectSide[] = ['left', 'right', 'top', 'bottom']

const gatesOf = (g: RouteGrid, r: Rect, sides: readonly RectSide[]): Gate[] => {
  const nx = g.xs.length
  const [l, rr] = nearest(g.xs, r.x, r.x + r.w)
  const [a, b] = nearest(g.ys, r.y, r.y + r.h)
  const cx = r.x + r.w / 2
  const cy = r.y + r.h / 2
  const gates: Gate[] = []
  for (const [ix, sideX, out, name] of [
    [l, r.x, -1, 'left'],
    [rr, r.x + r.w, 1, 'right'],
  ] as const) {
    if (ix < 0 || !sides.includes(name)) continue
    const stub = Math.abs(g.xs[ix].at - sideX)
    const side = (iy: number) => ({ ix, iy, out, isTopOrBottom: false })
    if (a >= 0) gates.push({ node: a * nx + ix, dir: 3, cost: stub + (cy - g.ys[a].at), side: side(a) })
    if (b >= 0) gates.push({ node: b * nx + ix, dir: 2, cost: stub + (g.ys[b].at - cy), side: side(b) })
  }
  for (const [iy, sideY, out, name] of [
    [a, r.y, -1, 'top'],
    [b, r.y + r.h, 1, 'bottom'],
  ] as const) {
    if (iy < 0 || !sides.includes(name)) continue
    const stub = Math.abs(g.ys[iy].at - sideY)
    const side = (ix: number) => ({ ix, iy, out, isTopOrBottom: true })
    if (l >= 0) gates.push({ node: iy * nx + l, dir: 1, cost: stub + (cx - g.xs[l].at), side: side(l) })
    if (rr >= 0) gates.push({ node: iy * nx + rr, dir: 0, cost: stub + (g.xs[rr].at - cx), side: side(rr) })
  }
  return gates
}

const segKey = (node: number, axis: number) => node * 2 + axis

interface Scratch {
  dist: Float64Array
  queued: Float64Array
  prev: Int32Array
  seen: Uint32Array
  gen: number
}

interface Usage {
  bundlesOn: (number[] | undefined)[]
  routed: Set<number>
}

const search = (
  g: RouteGrid,
  starts: Gate[],
  goals: Gate[],
  usage: Usage,
  bundle: number,
  bend: number,
  pull: number,
  scratch: Scratch,
): { nodes: number[]; start: Gate; goal: Gate } | null => {
  const nx = g.xs.length
  const ny = g.ys.length
  const { dist, queued, prev, seen } = scratch
  const gen = ++scratch.gen
  const distOf = (st: number) => (seen[st] === gen ? dist[st] : Infinity)
  const reach = (st: number, d: number, from: number) => {
    seen[st] = gen
    dist[st] = d
    prev[st] = from
  }
  const xAt = Float64Array.from(g.xs, (l) => l.at)
  const yAt = Float64Array.from(g.ys, (l) => l.at)
  const goalI = Int32Array.from(goals, (gt) => gt.node % nx)
  const goalJ = Int32Array.from(goals, (gt) => Math.floor(gt.node / nx))
  const h = (node: number, dir: number) => {
    const i = node % nx
    const j = (node - i) / nx
    const x = xAt[i]
    const y = yAt[j]
    const isHorizontal = dir < 2
    const sign = dir % 2 === 0 ? 1 : -1
    let best = Infinity
    for (let k = 0; k < goalI.length; k++) {
      const gi = goalI[k]
      const gj = goalJ[k]
      const ahead = isHorizontal ? (gi - i) * sign : (gj - j) * sign
      const isInLine = isHorizontal ? gj === j : gi === i
      const turns = isInLine ? (ahead < 0 ? 2 : 0) : ahead < 0 ? 2 : 1
      const est = (Math.abs(xAt[gi] - x) + Math.abs(yAt[gj] - y)) * pull + turns * bend
      if (est < best) best = est
    }
    return best
  }
  const heap = new Heap()
  const push = (key: number, st: number) => {
    queued[st] = key
    heap.push(key, st)
  }
  const { bundlesOn } = usage
  const crossingAt = (i: number, j: number, horizontal: boolean): number => {
    if (horizontal ? j <= 0 || j >= ny - 1 : i <= 0 || i >= nx - 1) return 0
    const before = bundlesOn[horizontal ? segKey((j - 1) * nx + i, 1) : segKey(j * nx + i - 1, 0)]
    const after = before && bundlesOn[horizontal ? segKey(j * nx + i, 1) : segKey(j * nx + i, 0)]
    if (!before || !after) return 0
    for (const b of before) if (b !== bundle && after.includes(b)) return CROSS
    return 0
  }
  for (const s of starts) {
    const st = s.node * 4 + s.dir
    if (s.cost < distOf(st)) {
      reach(st, s.cost, -1)
      push(s.cost + h(s.node, s.dir), st)
    }
  }
  const origin = new Map(starts.map((s) => [s.node * 4 + s.dir, s]))
  let best = Infinity
  let bestState = -1
  let bestGoal: Gate | null = null
  let expanded = 0
  while (heap.size && expanded++ < MAX_EXPANDED) {
    const key = heap.top
    if (key >= best) break
    const st = heap.pop()
    if (key !== queued[st]) continue
    const d = dist[st]
    const node = st >> 2
    const dir = st & 3
    for (const goal of goals) {
      if (goal.node !== node || dir === goal.dir) continue
      const total = d + goal.cost + (dir === (goal.dir ^ 1) ? 0 : bend)
      if (total < best) {
        best = total
        bestState = st
        bestGoal = goal
      }
    }
    const i = node % nx
    const j = (node - i) / nx
    for (let nd = 0; nd < 4; nd++) {
      if (nd === (dir ^ 1)) continue
      const dx = DIRS[nd][0]
      const dy = DIRS[nd][1]
      const ni = i + dx
      const nj = j + dy
      if (ni < 0 || nj < 0 || ni >= nx || nj >= ny) continue
      const horizontal = dy === 0
      const lo = horizontal ? j * nx + Math.min(i, ni) : Math.min(j, nj) * nx + i
      if (!(horizontal ? g.freeH[lo] : g.freeV[lo])) continue
      const cost = horizontal ? g.ys[j].cost : g.xs[i].cost
      const len = horizontal ? Math.abs(xAt[ni] - xAt[i]) : Math.abs(yAt[nj] - yAt[j])
      const seg = segKey(lo, horizontal ? 0 : 1)
      const users = bundlesOn[seg]
      const factor = !users ? 1 : users.includes(bundle) ? SHARED : 1 + users.length * CROWDED
      const next = nj * nx + ni
      const ns = next * 4 + nd
      const nd2 = d + len * cost * factor + (nd === dir ? 0 : bend) + crossingAt(ni, nj, horizontal)
      if (nd2 < distOf(ns)) {
        reach(ns, nd2, st)
        push(nd2 + h(next, nd), ns)
      }
    }
  }
  if (!bestGoal) return null
  const states: number[] = []
  for (let s = bestState; s >= 0; s = prev[s]) states.push(s)
  states.reverse()
  return { nodes: states.map((s) => s >> 2), start: origin.get(states[0])!, goal: bestGoal }
}

interface Run {
  axis: 0 | 1
  line: number
  a: number
  b: number
}

interface Request {
  key: string
  from: number
  to: number
  bundle: string
  hub: number | null
  sides: readonly RectSide[]
  joinAt?: 'from' | 'to'
}

interface Routed {
  req: Request
  runs: Run[]
  startSide: Side
  endSide: Side
}

const runsOf = (g: RouteGrid, nodes: number[]): Run[] => {
  const nx = g.xs.length
  const runs: Run[] = []
  for (let k = 0; k + 1 < nodes.length; k++) {
    const [i1, j1] = [nodes[k] % nx, Math.floor(nodes[k] / nx)]
    const [i2, j2] = [nodes[k + 1] % nx, Math.floor(nodes[k + 1] / nx)]
    const axis: 0 | 1 = j1 === j2 ? 0 : 1
    const line = axis === 0 ? j1 : i1
    const a = axis === 0 ? i1 : j1
    const b = axis === 0 ? i2 : j2
    const last = runs[runs.length - 1]
    if (last && last.axis === axis && last.line === line) last.b = b
    else runs.push({ axis, line, a, b })
  }
  return runs
}

const markUsage = (usage: Usage, g: RouteGrid, nodes: number[], bundle: number) => {
  const nx = g.xs.length
  usage.routed.add(bundle)
  for (let k = 0; k + 1 < nodes.length; k++) {
    const [p, q] = [nodes[k], nodes[k + 1]]
    const horizontal = Math.floor(p / nx) === Math.floor(q / nx)
    const key = segKey(Math.min(p, q), horizontal ? 0 : 1)
    const users = usage.bundlesOn[key]
    if (!users) usage.bundlesOn[key] = [bundle]
    else if (!users.includes(bundle)) users.push(bundle)
  }
}

export const roundedPath = (raw: readonly Point[], radius = CORNER_PX): { d: string; points: Point[] } => {
  const clean = raw.filter((p, i) => i === 0 || Math.abs(p.x - raw[i - 1].x) > 0.01 || Math.abs(p.y - raw[i - 1].y) > 0.01)
  const keep = clean.filter((p, i) => {
    if (i === 0 || i === clean.length - 1) return true
    const [a, c] = [clean[i - 1], clean[i + 1]]
    return Math.abs((p.x - a.x) * (c.y - p.y) - (p.y - a.y) * (c.x - p.x)) > 0.01
  })
  const r = (v: number) => Math.round(v * 10) / 10
  let d = `M${r(keep[0].x)},${r(keep[0].y)}`
  for (let i = 1; i < keep.length - 1; i++) {
    const [a, p, c] = [keep[i - 1], keep[i], keep[i + 1]]
    const inLen = Math.hypot(p.x - a.x, p.y - a.y)
    const outLen = Math.hypot(c.x - p.x, c.y - p.y)
    const rad = Math.min(radius, inLen / 2, outLen / 2)
    const p1 = { x: p.x - ((p.x - a.x) / inLen) * rad, y: p.y - ((p.y - a.y) / inLen) * rad }
    const p2 = { x: p.x + ((c.x - p.x) / outLen) * rad, y: p.y + ((c.y - p.y) / outLen) * rad }
    d += ` L${r(p1.x)},${r(p1.y)} Q${r(p.x)},${r(p.y)} ${r(p2.x)},${r(p2.y)}`
  }
  const last = keep[keep.length - 1]
  return { d: `${d} L${r(last.x)},${r(last.y)}`, points: keep }
}

const BRANCH_LABEL_PX = 18

export const alongRun = (a: Point, b: Point, px: number): Point => {
  const len = Math.hypot(b.x - a.x, b.y - a.y)
  const k = len ? Math.min(px, len / 2) / len : 0
  return { x: a.x + (b.x - a.x) * k, y: a.y + (b.y - a.y) * k }
}

export const routeAround = (links: readonly RouteLink[], options: RouteOptions): Map<string, Route> => {
  const ids = new Map<Rect, number>()
  const cells = new Map<number, Rect>()
  const idOf = (rect: Rect): number => {
    const known = ids.get(rect)
    if (known !== undefined) return known
    ids.set(rect, ids.size)
    cells.set(ids.size - 1, rect)
    return ids.size - 1
  }
  const reqs = links
    .map((l, i) => ({ l, i }))
    .sort((a, b) => (b.l.weight ?? 0) - (a.l.weight ?? 0) || a.i - b.i)
    .map(({ l }): Request => {
      const from = idOf(l.from)
      const to = idOf(l.to)
      return {
        key: l.key,
        from,
        to,
        bundle: l.bundle ?? l.key,
        hub: l.joinAt === 'from' ? from : l.joinAt === 'to' ? to : null,
        sides: l.sides ?? ALL_SIDES,
        joinAt: l.joinAt,
      }
    })
  const lines = options.lines ?? gutterLines(options.obstacles)
  const g = routeGrid(lines.xs, lines.ys, options.obstacles)
  const track = options.track ?? TRACK_PX
  const usage: Usage = { bundlesOn: [], routed: new Set() }
  const bundleIds = new Map<string, number>()
  const bundleId = (bundle: string): number => {
    const known = bundleIds.get(bundle)
    if (known !== undefined) return known
    bundleIds.set(bundle, bundleIds.size)
    return bundleIds.size - 1
  }
  const states = g.xs.length * g.ys.length * 4
  const scratch: Scratch = {
    dist: new Float64Array(states),
    queued: new Float64Array(states),
    prev: new Int32Array(states),
    seen: new Uint32Array(states),
    gen: 0,
  }
  const routed: Routed[] = []
  for (const req of reqs) {
    const a = cells.get(req.from)!
    const b = cells.get(req.to)!
    const id = bundleId(req.bundle)
    const pull = req.hub !== null && usage.routed.has(id) ? SHARED : 1
    const found = search(g, gatesOf(g, a, req.sides), gatesOf(g, b, req.sides), usage, id, options.bend ?? BEND, pull, scratch)
    if (!found) continue
    markUsage(usage, g, found.nodes, id)
    routed.push({
      req,
      runs: runsOf(g, found.nodes),
      startSide: found.start.side,
      endSide: found.goal.side,
    })
  }

  const centre = (id: number) => {
    const c = cells.get(id)!
    return [c.x + c.w / 2, c.y + c.h / 2]
  }
  const otherOf = (r: Routed, atStart: boolean) => centre(atStart ? r.req.to : r.req.from)

  const ports = new Map<string, { key: string; y: number; x: number }[]>()
  const endOf = (r: Routed, atStart: boolean) => (atStart ? r.req.from : r.req.to)
  const sideOf = (r: Routed, atStart: boolean) => (atStart ? r.startSide : r.endSide)
  const portKey = (r: Routed, atStart: boolean) => {
    const side = sideOf(r, atStart)
    return `${endOf(r, atStart)}:${side.isTopOrBottom ? 'v' : 'h'}${side.out}`
  }
  const attachKey = (r: Routed, atStart: boolean) =>
    endOf(r, atStart) === r.req.hub ? `hub|${r.req.bundle}` : `${atStart ? 's' : 'e'}|${r.req.key}`
  for (const r of routed) {
    for (const atStart of [true, false]) {
      const pk = portKey(r, atStart)
      const list = ports.get(pk) ?? []
      const ak = attachKey(r, atStart)
      if (!list.some((p) => p.key === ak)) {
        const [x, y] = otherOf(r, atStart)
        list.push({ key: ak, x, y })
      }
      ports.set(pk, list)
    }
  }
  const portAt = new Map<string, number>()
  for (const [pk, list] of ports) {
    const [id, kind] = pk.split(':')
    const c = cells.get(Number(id))!
    if (kind.startsWith('v')) {
      list.sort((p, q) => p.x - q.x || p.y - q.y || (p.key < q.key ? -1 : 1))
      list.forEach((p, k) => portAt.set(`${pk}|${p.key}`, c.x + ((k + 1) * c.w) / (list.length + 1)))
    } else {
      list.sort((p, q) => p.y - q.y || p.x - q.x || (p.key < q.key ? -1 : 1))
      list.forEach((p, k) => portAt.set(`${pk}|${p.key}`, c.y + ((k + 1) * c.h) / (list.length + 1)))
    }
  }

  const lineUsers = new Map<string, Map<string, number[]>>()
  const linePorts = new Map<string, Map<string, number[]>>()
  const lineRoom = new Map<string, number>()
  for (const r of routed) {
    const stubs: [Side, Rect, number][] = [true, false].map((atStart) => [
      sideOf(r, atStart),
      cells.get(endOf(r, atStart))!,
      portAt.get(`${portKey(r, atStart)}|${attachKey(r, atStart)}`)!,
    ])
    const [from, to] = [centre(r.req.from), centre(r.req.to)]
    const across = (run: Run, k: number) => {
      const axis = run.axis === 0 ? 1 : 0
      const lines = run.axis === 0 ? g.ys : g.xs
      const before = k > 0 ? lines[r.runs[k - 1].a].at : from[axis]
      const after = k < r.runs.length - 1 ? lines[r.runs[k + 1].b].at : to[axis]
      return (before + after) / 2
    }
    const spans = [
      ...r.runs.map((run, k) => {
        const along = run.axis === 0 ? g.xs : g.ys
        const lo = Math.min(along[run.a].at, along[run.b].at)
        return { run, lo, hi: Math.max(along[run.a].at, along[run.b].at), key: across(run, k) }
      }),
      ...stubs.map(([side, cell, port]) => {
        const far = side.isTopOrBottom ? g.xs[side.ix].at : g.ys[side.iy].at
        const run: Run = side.isTopOrBottom
          ? { axis: 0, line: side.iy, a: side.ix, b: side.ix }
          : { axis: 1, line: side.ix, a: side.iy, b: side.iy }
        const [lo, hi] = side.isTopOrBottom ? [cell.x, cell.x + cell.w] : [cell.y, cell.y + cell.h]
        return { run, lo: Math.min(far, lo), hi: Math.max(far, hi), port: side.out * (far < port ? port : -port) }
      }),
    ]
    for (const span of spans) {
      const { run, lo, hi } = span
      const key = `${run.axis}:${run.line}`
      const into = 'port' in span ? linePorts : lineUsers
      const users = into.get(key) ?? new Map<string, number[]>()
      users.set(r.req.bundle, [...(users.get(r.req.bundle) ?? []), 'port' in span ? span.port : span.key])
      into.set(key, users)
      if (!lineUsers.has(key)) lineUsers.set(key, new Map())
      const at = (run.axis === 0 ? g.ys : g.xs)[run.line].at
      lineRoom.set(key, Math.min(lineRoom.get(key) ?? Infinity, clearance(g, run.axis, at, lo, hi)))
    }
  }
  const mean = (v: number[]) => v.reduce((s, x) => s + x, 0) / v.length
  const offsets = new Map<string, number>()
  for (const [key, passing] of lineUsers) {
    const [axis, line] = key.split(':').map(Number)
    const at = (axis === 0 ? g.ys : g.xs)[line].at
    const room = Math.min((axis === 0 ? g.ys : g.xs)[line].room, lineRoom.get(key) ?? 0)
    const ported = linePorts.get(key) ?? new Map<string, number[]>()
    const bundles = new Set([...passing.keys(), ...ported.keys()])
    const sortKey = (b: string) => {
      const p = ported.get(b)
      return p ? at + mean(p) * PORT_KEY_SCALE : mean(passing.get(b)!)
    }
    const order = [...bundles]
      .map((b): [string, number] => [b, sortKey(b)])
      .sort((p, q) => p[1] - q[1] || (p[0] < q[0] ? -1 : 1))
    const n = order.length
    const step = n > 1 ? Math.min(track, (2 * Math.max(0, room)) / (n - 1)) : 0
    order.forEach(([bundle], k) => offsets.set(`${key}|${bundle}`, (k - (n - 1) / 2) * step))
  }
  const offsetOf = (axis: number, line: number, bundle: string) => offsets.get(`${axis}:${line}|${bundle}`) ?? 0

  const out = new Map<string, Route>()
  const labels: Point[] = []
  for (const r of routed) {
    const at = (axis: number, line: number) =>
      (axis === 0 ? g.ys : g.xs)[line].at + offsetOf(axis, line, r.req.bundle)
    const ends = [true, false].map((atStart) => {
      const cell = cells.get(endOf(r, atStart))!
      const side = sideOf(r, atStart)
      const along = portAt.get(`${portKey(r, atStart)}|${attachKey(r, atStart)}`)!
      const gate: [number, number] = side.isTopOrBottom ? [along, at(0, side.iy)] : [at(1, side.ix), along]
      const edge = side.out > 0 ? (side.isTopOrBottom ? cell.y + cell.h : cell.x + cell.w) : side.isTopOrBottom ? cell.y : cell.x
      const port: [number, number] = side.isTopOrBottom ? [along, edge] : [edge, along]
      return { side, port, gate }
    })
    const [start, end] = ends
    const pts: [number, number][] = [start.port, start.gate]
    let [x, y] = start.gate
    for (const run of r.runs) {
      if (run.axis === 0) {
        y = at(0, run.line)
        pts.push([x, y])
        x = at(1, run.b)
        pts.push([x, y])
      } else {
        x = at(1, run.line)
        pts.push([x, y])
        y = at(0, run.b)
        pts.push([x, y])
      }
    }
    if (end.side.isTopOrBottom) pts.push([x, end.gate[1]], end.gate, end.port)
    else pts.push([end.gate[0], y], end.gate, end.port)
    const path = roundedPath(
      pts.map(([px, py]) => ({ x: px, y: py })),
      options.corner ?? CORNER_PX,
    )
    const p = path.points
    const label =
      r.req.joinAt === 'to'
        ? alongRun(p[0], p[1], BRANCH_LABEL_PX)
        : r.req.joinAt === 'from'
          ? alongRun(p[p.length - 1], p[p.length - 2], BRANCH_LABEL_PX)
          : labelSpot(pts, labels)
    out.set(r.req.key, { d: path.d, points: p, label })
  }
  return out
}

const LABEL_GAP_PX = 18

const labelSpot = (pts: [number, number][], placed: Point[]): Point => {
  const spans = pts
    .slice(1)
    .map((p, i) => ({ a: pts[i], b: p, len: Math.abs(p[0] - pts[i][0]) + Math.abs(p[1] - pts[i][1]) }))
    .sort((p, q) => q.len - p.len)
  const mid = (s: (typeof spans)[number]): Point => ({ x: (s.a[0] + s.b[0]) / 2, y: (s.a[1] + s.b[1]) / 2 })
  const free = spans.map(mid).find((m) => placed.every((p) => Math.hypot(p.x - m.x, p.y - m.y) >= LABEL_GAP_PX))
  const spot = free ?? mid(spans[0])
  placed.push(spot)
  return spot
}

export interface Curve {
  d: string
  label: Point
}

const CURVE = { maxBend: 80, bendShare: 0.2, arrowClearance: 6 } as const

const clip = (from: Point, toward: Point, half: { w: number; h: number }): Point => {
  const dx = toward.x - from.x
  const dy = toward.y - from.y
  const t = Math.min(half.w / Math.abs(dx || Number.EPSILON), half.h / Math.abs(dy || Number.EPSILON), 1)
  return { x: from.x + dx * t, y: from.y + dy * t }
}

const centreOf = (b: Rect): Point => ({ x: b.x + b.w / 2, y: b.y + b.h / 2 })

export const curveBetween = (a: Rect, b: Rect): Curve => {
  const p = centreOf(a)
  const q = centreOf(b)
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
