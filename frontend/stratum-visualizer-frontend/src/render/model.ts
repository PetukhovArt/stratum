import {
  ContainerKind,
  EdgeKind,
  GraphSnapshot,
  SnapshotContainer,
  SnapshotEdge,
  SnapshotLayer,
  SnapshotModule,
  SnapshotZone,
  Territory,
  Violation,
} from '../types'
import { layerHue, SeverityLabel } from './encoding'

export interface Group {
  key: string
  kind: ContainerKind | 'private'
  container: number
  zone: number | null
  name: string
  path: string
  layer: number
}

export interface Unit {
  id: number
  kind: ContainerKind
  layer: number
  name: string
  path: string
  modules: number[]
}

export interface AggregateEdge {
  from: number
  to: number
  count: number
  findings: number
  rules: string[]
  worst: SeverityLabel | null
}

export interface Model {
  project: string
  layers: readonly SnapshotLayer[]
  containers: ReadonlyMap<number, SnapshotContainer>
  modules: ReadonlyMap<number, SnapshotModule>
  zones: readonly SnapshotZone[]
  edges: readonly SnapshotEdge[]
  violations: readonly Violation[]
  byPath: ReadonlyMap<string, number>
  out: (id: number) => readonly number[]
  inn: (id: number) => readonly number[]
  edge: (from: number, to: number) => SnapshotEdge | undefined
  findingsOnEdge: (from: number, to: number) => readonly Violation[]
  findingsOf: (id: number) => readonly Violation[]
  territoryOf: (id: number) => Territory
  groupOf: (id: number) => Group
  /** Containers enclosing the module, outermost first. */
  chain: (id: number) => readonly SnapshotContainer[]
  label: (id: number) => string
  /** The deepest enclosing container that is not `nested`. */
  unitOf: (id: number) => number
  units: readonly Unit[]
  unitEdges: readonly AggregateEdge[]
  layerEdges: readonly AggregateEdge[]
  /** Unit cycles of two or more units, largest first. */
  unitSccs: readonly (readonly number[])[]
  layerHue: (index: number) => number
  worst: (violations: readonly Violation[]) => SeverityLabel | null
}

const SEVERITY_RANK: Record<string, number> = { error: 3, warning: 2, info: 1 }

export const worst = (violations: readonly Violation[]): SeverityLabel | null => {
  let best: SeverityLabel | null = null
  for (const v of violations) {
    const rank = SEVERITY_RANK[v.severity] ?? 0
    if (rank > (best ? SEVERITY_RANK[best] : 0)) {
      best = v.severity as SeverityLabel
    }
  }
  return best
}

export const sccs = <T>(nodes: Iterable<T>, adj: (node: T) => Iterable<T>): T[][] => {
  const index = new Map<T, number>()
  const low = new Map<T, number>()
  const onStack = new Set<T>()
  const stack: T[] = []
  const result: T[][] = []
  let counter = 0

  const visit = (root: T): void => {
    const frames: { node: T; next: Iterator<T> }[] = []
    const enter = (node: T): void => {
      index.set(node, counter)
      low.set(node, counter)
      counter++
      stack.push(node)
      onStack.add(node)
      frames.push({ node, next: adj(node)[Symbol.iterator]() })
    }
    enter(root)
    while (frames.length) {
      const frame = frames[frames.length - 1]
      const step = frame.next.next()
      if (!step.done) {
        const w = step.value
        if (!index.has(w)) {
          enter(w)
        } else if (onStack.has(w)) {
          low.set(frame.node, Math.min(low.get(frame.node)!, index.get(w)!))
        }
        continue
      }
      frames.pop()
      const v = frame.node
      const parent = frames[frames.length - 1]
      if (parent) {
        low.set(parent.node, Math.min(low.get(parent.node)!, low.get(v)!))
      }
      if (low.get(v) !== index.get(v)) continue
      const component: T[] = []
      let w: T
      do {
        w = stack.pop()!
        onStack.delete(w)
        component.push(w)
      } while (w !== v)
      result.push(component)
    }
  }

  for (const n of nodes) {
    if (!index.has(n)) visit(n)
  }
  return result
}

const GENERIC_BASENAME =
  /^(index|types|utils|config|actions|mutations|getters|routes|store|strategy|settings|converters|constants|helpers)\./

const edgeKey = (from: number, to: number): string => `${from}>${to}`

const push = <K, V>(map: Map<K, V[]>, key: K, value: V): void => {
  const list = map.get(key)
  if (list) {
    list.push(value)
  } else {
    map.set(key, [value])
  }
}

const NONE: readonly never[] = []

const aggregate = (
  edges: readonly SnapshotEdge[],
  endpoint: (moduleId: number) => number,
  findingsOnEdge: (from: number, to: number) => readonly Violation[],
): AggregateEdge[] => {
  const byKey = new Map<string, { from: number; to: number; count: number; found: Violation[] }>()
  for (const e of edges) {
    const from = endpoint(e.from)
    const to = endpoint(e.to)
    if (from === to) continue
    const key = edgeKey(from, to)
    let agg = byKey.get(key)
    if (!agg) {
      agg = { from, to, count: 0, found: [] }
      byKey.set(key, agg)
    }
    agg.count++
    agg.found.push(...findingsOnEdge(e.from, e.to))
  }
  return [...byKey.values()].map(({ found, ...agg }) => ({
    ...agg,
    findings: found.length,
    rules: [...new Set(found.map((v) => v.rule))].sort(),
    worst: worst(found),
  }))
}

export const buildModel = (snapshot: GraphSnapshot, violations: readonly Violation[]): Model => {
  const modules = new Map(snapshot.modules.map((m) => [m.id, m]))
  const containers = new Map(snapshot.containers.map((c) => [c.id, c]))
  const byPath = new Map(snapshot.modules.map((m) => [m.path, m.id]))

  const moduleOf = (id: number): SnapshotModule => {
    const m = modules.get(id)
    if (!m) throw new Error(`Unknown module id ${id}`)
    return m
  }

  const edgeByKey = new Map<string, SnapshotEdge>()
  const outMap = new Map<number, number[]>()
  const innMap = new Map<number, number[]>()
  for (const e of snapshot.edges) {
    const key = edgeKey(e.from, e.to)
    const seen = edgeByKey.get(key)
    if (e.from === e.to) continue
    if (seen) {
      if (e.kind !== EdgeKind.Static) edgeByKey.set(key, { ...seen, kind: e.kind })
      continue
    }
    edgeByKey.set(key, e)
    push(outMap, e.from, e.to)
    push(innMap, e.to, e.from)
  }
  const uniqueEdges = [...edgeByKey.values()]

  const findingsByEdge = new Map<string, Violation[]>()
  const findingsByModule = new Map<number, Violation[]>()
  for (const v of violations) {
    const touched = new Set(v.modules)
    if (v.edge) {
      push(findingsByEdge, edgeKey(v.edge.from, v.edge.to), v)
      touched.add(v.edge.from)
      touched.add(v.edge.to)
    }
    for (const id of touched) push(findingsByModule, id, v)
  }
  const findingsOnEdge = (from: number, to: number) => findingsByEdge.get(edgeKey(from, to)) ?? NONE

  const chainCache = new Map<number, SnapshotContainer[]>()
  const containerChain = (containerId: number): SnapshotContainer[] => {
    const cached = chainCache.get(containerId)
    if (cached) return cached
    const out: SnapshotContainer[] = []
    const seen = new Set<number>()
    let cur = containers.get(containerId)
    while (cur && !seen.has(cur.id)) {
      seen.add(cur.id)
      out.unshift(cur)
      cur = cur.parent === null ? undefined : containers.get(cur.parent)
    }
    chainCache.set(containerId, out)
    return out
  }
  const chain = (id: number) => containerChain(moduleOf(id).container)

  const unitOfContainer = (containerId: number): number => {
    const units = containerChain(containerId).filter((c) => c.kind !== 'nested')
    return units[units.length - 1]?.id ?? containerId
  }
  const unitOf = (id: number) => unitOfContainer(moduleOf(id).container)

  const groupCache = new Map<number, Group>()
  const groupOf = (id: number): Group => {
    const cached = groupCache.get(id)
    if (cached) return cached
    const m = moduleOf(id)
    const inner = containers.get(m.container)
    const zone = typeof m.zone === 'number' ? snapshot.zones[m.zone] : undefined
    const innerPath = inner?.path ?? ''
    const group: Group = zone
      ? {
          key: `z${zone.id}`,
          kind: 'private',
          container: m.container,
          zone: zone.id,
          name: `${inner?.name ?? ''}/${zone.path.slice(innerPath.length + 1)}`,
          path: zone.path,
          layer: m.layer,
        }
      : {
          key: `c${m.container}`,
          kind: inner?.kind ?? 'layer',
          container: m.container,
          zone: null,
          name: inner?.name ?? '',
          path: innerPath,
          layer: m.layer,
        }
    groupCache.set(id, group)
    return group
  }

  const label = (id: number): string => {
    const segs = moduleOf(id).path.split('/')
    const base = segs.pop() ?? ''
    const parent = segs.pop()
    return GENERIC_BASENAME.test(base) && parent ? `${parent}/${base}` : base
  }

  const units: Unit[] = snapshot.containers
    .filter((c) => c.kind !== 'nested')
    .map((c) => ({ id: c.id, kind: c.kind, layer: c.layer, name: c.name, path: c.path, modules: [] }))
  const unitById = new Map(units.map((u) => [u.id, u]))
  for (const m of snapshot.modules) unitById.get(unitOfContainer(m.container))?.modules.push(m.id)

  const unitEdges = aggregate(uniqueEdges, unitOf, findingsOnEdge)
  const unitOut = new Map<number, number[]>()
  for (const e of unitEdges) push(unitOut, e.from, e.to)
  const unitSccs = sccs(
    units.map((u) => u.id),
    (u) => unitOut.get(u) ?? NONE,
  )
    .filter((c) => c.length > 1)
    .sort((a, b) => b.length - a.length)

  return {
    project: snapshot.project,
    layers: snapshot.layers,
    containers,
    modules,
    zones: snapshot.zones,
    edges: uniqueEdges,
    violations,
    byPath,
    out: (id) => outMap.get(id) ?? NONE,
    inn: (id) => innMap.get(id) ?? NONE,
    edge: (from, to) => edgeByKey.get(edgeKey(from, to)),
    findingsOnEdge,
    findingsOf: (id) => findingsByModule.get(id) ?? NONE,
    territoryOf: (id) => snapshot.territories[moduleOf(id).territory],
    groupOf,
    chain,
    label,
    unitOf,
    units,
    unitEdges,
    layerEdges: aggregate(uniqueEdges, (id) => moduleOf(id).layer, findingsOnEdge),
    unitSccs,
    layerHue: (index) => layerHue(index, snapshot.layers.length),
    worst,
  }
}
