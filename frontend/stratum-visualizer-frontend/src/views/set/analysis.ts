import { edgeKey, Model, sccs } from '../../render/model'

export const MAX_PATH_HOPS = 4
export const VIA_LIMIT = 24
export const SET_LIMIT = 40

export interface SetEdge {
  from: number
  to: number
  /** True for an import on a connecting path; false for an import between two members. */
  isPath: boolean
}

export interface Cut {
  from: number
  to: number
  /** Loop members that leave the largest cycle when this import is removed. */
  freed: number
}

export interface SetAnalysis {
  members: readonly number[]
  via: readonly number[]
  edges: readonly SetEdge[]
  /** Each path runs member → non-members → member, at most MAX_PATH_HOPS imports. */
  paths: readonly (readonly number[])[]
  loop: readonly number[] | null
  cuts: readonly Cut[]
  /** Closed: [cut.from, cut.to, …, cut.from]. */
  chain: readonly number[] | null
  order: readonly number[]
}


const byLayerThenPath = (m: Model) => (a: number, b: number) =>
  m.modules.get(a)!.layer - m.modules.get(b)!.layer || m.modules.get(a)!.path.localeCompare(m.modules.get(b)!.path)

const reachInside = (m: Model, from: number, inSet: ReadonlySet<number>): Set<number> => {
  const seen = new Set([from])
  const queue = [from]
  for (let i = 0; i < queue.length; i++) {
    for (const next of m.out(queue[i])) {
      if (!inSet.has(next) || seen.has(next)) continue
      seen.add(next)
      queue.push(next)
    }
  }
  return seen
}

const parentsThroughOutsiders = (m: Model, from: number, inSet: ReadonlySet<number>): Map<number, number> => {
  const parent = new Map<number, number>([[from, from]])
  let frontier = [from]
  for (let hop = 0; hop < MAX_PATH_HOPS && frontier.length; hop++) {
    const next: number[] = []
    for (const x of frontier) {
      for (const y of m.out(x)) {
        if (parent.has(y)) continue
        parent.set(y, x)
        if (!inSet.has(y)) next.push(y)
      }
    }
    frontier = next
  }
  return parent
}

const walkBack = (parent: ReadonlyMap<number, number>, from: number, to: number): number[] => {
  const path = [to]
  for (let cur = to; cur !== from; ) {
    cur = parent.get(cur)!
    path.unshift(cur)
  }
  return path
}

const connectingPaths = (m: Model, members: readonly number[], inSet: ReadonlySet<number>): number[][] => {
  const reach = new Map(members.map((a) => [a, reachInside(m, a, inSet)]))
  const paths: number[][] = []
  const via = new Set<number>()
  for (const a of members) {
    const isApart = (b: number) => a !== b && !reach.get(a)!.has(b) && !reach.get(b)!.has(a)
    const targets = members.filter(isApart)
    if (!targets.length) continue
    const parent = parentsThroughOutsiders(m, a, inSet)
    for (const b of targets) {
      if (!parent.has(b)) continue
      const path = walkBack(parent, a, b)
      paths.push(path)
      for (const x of path.slice(1, -1)) via.add(x)
      if (via.size > VIA_LIMIT) return paths
    }
  }
  return paths
}

const largestCycleSize = (nodes: readonly number[], adj: (id: number) => Iterable<number>): number =>
  sccs(nodes, adj).reduce((best, c) => (c.length > 1 ? Math.max(best, c.length) : best), 0)

const cutsOf = (m: Model, loop: readonly number[]): Cut[] => {
  const inLoop = new Set(loop)
  const inside = (x: number) => m.out(x).filter((y) => inLoop.has(y))
  const cuts = loop.flatMap((from) =>
    inside(from).map((to) => {
      const without = (x: number) => (x === from ? inside(x).filter((y) => y !== to) : inside(x))
      return { from, to, freed: loop.length - largestCycleSize(loop, without) }
    }),
  )
  return cuts.sort((a, b) => b.freed - a.freed)
}

const shortestInside = (m: Model, from: number, to: number, inside: ReadonlySet<number>): number[] | null => {
  const parent = new Map<number, number>([[from, from]])
  const queue = [from]
  for (let i = 0; i < queue.length; i++) {
    const x = queue[i]
    if (x === to) return walkBack(parent, from, to)
    for (const y of m.out(x)) {
      if (!inside.has(y) || parent.has(y)) continue
      parent.set(y, x)
      queue.push(y)
    }
  }
  return null
}

const loopOrder = (m: Model, loop: readonly number[], chain: readonly number[] | null): number[] => {
  const inLoop = new Set(loop)
  const head = chain ? chain.slice(0, -1) : []
  const seen = new Set(head)
  const order = [...head]
  const stack = [chain ? chain[0] : loop[0]]
  while (stack.length) {
    const x = stack.pop()!
    if (!seen.has(x)) {
      seen.add(x)
      order.push(x)
    }
    const next = m.out(x).filter((y) => inLoop.has(y) && !seen.has(y))
    stack.push(...next.reverse())
  }
  return [...order, ...loop.filter((x) => !seen.has(x))]
}

export const analyzeSet = (m: Model, ids: readonly number[]): SetAnalysis => {
  const members = [...new Set(ids.filter((id) => m.modules.has(id)))]
  const inSet = new Set(members)
  const induced = members.flatMap((from) =>
    m.out(from)
      .filter((to) => inSet.has(to))
      .map((to): SetEdge => ({ from, to, isPath: false })),
  )
  const paths = connectingPaths(m, members, inSet)
  const pathEdges = new Map<string, SetEdge>()
  for (const p of paths) {
    for (let i = 0; i < p.length - 1; i++) {
      pathEdges.set(edgeKey(p[i], p[i + 1]), { from: p[i], to: p[i + 1], isPath: true })
    }
  }
  const via = [...new Set(paths.flatMap((p) => p.slice(1, -1)))]

  const cycles = sccs(members, (x) => m.out(x).filter((y) => inSet.has(y)))
    .filter((c) => c.length > 1)
    .sort((a, b) => b.length - a.length)
  const loop = cycles[0] ?? null
  const cuts = loop ? cutsOf(m, loop) : []
  const best = cuts[0]
  const back = loop && best ? shortestInside(m, best.to, best.from, new Set(loop)) : null
  const chain = back ? [best.from, ...back] : null

  const loopFirst = loop ? loopOrder(m, loop, chain) : []
  const placed = new Set(loopFirst)
  const rest = members.filter((id) => !placed.has(id)).sort(byLayerThenPath(m))
  const order = [...loopFirst, ...rest, ...[...via].sort(byLayerThenPath(m))]

  return { members, via, edges: [...induced, ...pathEdges.values()], paths, loop, cuts, chain, order }
}

export const bestCut = (a: SetAnalysis): Cut | null => (a.cuts[0] && a.cuts[0].freed > 0 ? a.cuts[0] : null)

const cyclesCache = new WeakMap<Model, readonly (readonly number[])[]>()

export const moduleCycles = (m: Model): readonly (readonly number[])[] => {
  const cached = cyclesCache.get(m)
  if (cached) return cached
  const found = sccs([...m.modules.keys()], m.out)
    .filter((c) => c.length > 1)
    .map((c) => c.sort((a, b) => a - b))
    .sort((a, b) => b.length - a.length || a[0] - b[0])
  cyclesCache.set(m, found)
  return found
}

export const withMember = (set: readonly number[], id: number): number[] => [...set.filter((x) => x !== id), id]
export const withoutMember = (set: readonly number[], id: number): number[] => set.filter((x) => x !== id)
