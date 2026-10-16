import { SeverityLabel } from '../../render/encoding'
import { Model } from '../../render/model'
import { EdgeKind, SnapshotContainer } from '../../types'
import { CycleChange, DriftReport, EdgeChange, StageChange } from './drift'

export type EdgeTag = 'same' | 'added' | 'removed'

export interface ReviewNode {
  /** Head module id; negative for a ghost, a module that exists only at the base. */
  id: number
  path: string
  isGhost: boolean
  /** `null` for a ghost whose path lies under no head container. */
  layer: number | null
  slice: number | null
  group: { key: string; name: string; path: string; container: number | null; isPrivate: boolean }
  isImporter: boolean
}

export interface ReviewEdge {
  key: string
  from: number
  to: number
  kind: EdgeKind
  tag: EdgeTag
  breaks: EdgeChange['breaks']
}

export type ChangeItem =
  | { kind: 'import'; edge: ReviewEdge; from: string; to: string }
  | { kind: 'cycle'; cycle: CycleChange }
  | { kind: 'stage'; stage: StageChange; container: number | null }

export interface FileRow {
  id: number
  path: string
  isGhost: boolean
  added: number
  removed: number
  worst: SeverityLabel | null
}

export interface Review {
  nodes: ReadonlyMap<number, ReviewNode>
  edges: readonly ReviewEdge[]
  files: readonly FileRow[]
  items: readonly ChangeItem[]
  stageOf: ReadonlyMap<number, StageChange>
  counts: { added: number; removed: number; breaking: number; cycles: number; stages: number }
  /** Changed imports left out of the region to stay within the mark budget. */
  omitted: number
}

/** Counted in module boxes + edges + change badges. */
export const MARK_BUDGET = 1300
const BADGE_MARKS = 1

const SEVERITY_RANK: Record<SeverityLabel, number> = { error: 3, warning: 2, info: 1 }

const edgeKey = (from: number, to: number): string => `${from}>${to}`

const isWorse = (a: SeverityLabel, b: SeverityLabel | null): boolean => !b || SEVERITY_RANK[a] > SEVERITY_RANK[b]

export const worstBreak = (breaks: EdgeChange['breaks']): SeverityLabel | null => {
  let best: SeverityLabel | null = null
  for (const [, severity] of breaks) {
    if (isWorse(severity, best)) best = severity
  }
  return best
}

const isUnder = (path: string, dir: string): boolean => path.startsWith(`${dir}/`)

const containerChain = (m: Model, id: number): SnapshotContainer[] => {
  const out: SnapshotContainer[] = []
  for (let c = m.containers.get(id); c; c = c.parent === null ? undefined : m.containers.get(c.parent)) {
    out.unshift(c)
  }
  return out
}

const sliceIn = (chain: readonly SnapshotContainer[]): number | null =>
  [...chain].reverse().find((c) => c.kind === 'slice')?.id ?? null

const headNode = (m: Model, id: number): ReviewNode => {
  const g = m.groupOf(id)
  return {
    id,
    path: m.modules.get(id)?.path ?? '',
    isGhost: false,
    layer: g.layer,
    slice: sliceIn(m.chain(id)),
    group: { key: g.key, name: g.name, path: g.path, container: g.container, isPrivate: g.kind === 'private' },
    isImporter: false,
  }
}

const ghostNode = (m: Model, id: number, path: string, byDepth: readonly SnapshotContainer[]): ReviewNode => {
  const home = byDepth.find((c) => isUnder(path, c.path))
  const group = home
    ? { key: `c${home.id}`, name: home.name, path: home.path, container: home.id, isPrivate: false }
    : { key: 'ghost', name: '', path: '', container: null, isPrivate: false }
  return {
    id,
    path,
    isGhost: true,
    layer: home?.layer ?? null,
    slice: home ? sliceIn(containerChain(m, home.id)) : null,
    group,
    isImporter: false,
  }
}

const representative = (m: Model, container: SnapshotContainer): number | null => {
  if (container.entry !== null) return container.entry
  let nested: number | null = null
  for (const mod of m.modules.values()) {
    if (mod.container === container.id) return mod.id
    if (nested === null && isUnder(mod.path, container.path)) nested = mod.id
  }
  return nested
}

export const toReview = (m: Model, drift: DriftReport): Review => {
  const byDepth = [...m.containers.values()].sort((a, b) => b.path.length - a.path.length)
  const containerByPath = new Map([...m.containers.values()].map((c) => [c.path, c]))
  const ghostIds = new Map<string, number>()
  const ghostPaths = new Map<number, string>()
  const idOf = (path: string): number => {
    const head = m.byPath.get(path) ?? ghostIds.get(path)
    if (head !== undefined) return head
    const ghost = -(ghostIds.size + 1)
    ghostIds.set(path, ghost)
    ghostPaths.set(ghost, path)
    return ghost
  }
  const pathOf = (id: number): string => m.modules.get(id)?.path ?? ghostPaths.get(id) ?? ''

  const changed = new Map<string, ReviewEdge>()
  const changeOf = drift.edges.map((e) => {
    const from = idOf(e.from)
    const to = idOf(e.to)
    const key = edgeKey(from, to)
    const made = changed.get(key) ?? { key, from, to, kind: e.kind, tag: e.change, breaks: e.breaks }
    changed.set(key, made)
    return made
  })
  const breaking = drift.breaking.map((i) => changeOf[i]).filter((e): e is ReviewEdge => e !== undefined)

  const cycleEdges: ReviewEdge[] = []
  for (const cycle of drift.cycles) {
    for (let i = 0; i + 1 < cycle.path.length; i++) {
      const from = idOf(cycle.path[i])
      const to = idOf(cycle.path[i + 1])
      const key = edgeKey(from, to)
      const head = m.edge(from, to)
      const edge = changed.get(key) ?? (head && { key, from, to, kind: head.kind, tag: 'same' as const, breaks: [] })
      if (edge) cycleEdges.push(edge)
    }
  }
  const all = [...changed.values()]
  const fixed = all.filter((e) => e.tag === 'removed' && e.breaks.length > 0)
  const clean = (tag: EdgeTag) => all.filter((e) => e.tag === tag && e.breaks.length === 0)
  const priority = [...breaking, ...cycleEdges, ...fixed, ...clean('added'), ...clean('removed')]

  const nodes = new Map<number, ReviewNode>()
  let marks = 0
  const addNode = (id: number): void => {
    if (nodes.has(id)) return
    const ghostPath = ghostPaths.get(id)
    nodes.set(id, ghostPath === undefined ? headNode(m, id) : ghostNode(m, id, ghostPath, byDepth))
    marks++
  }

  const stageOf = new Map<number, StageChange>()
  for (const stage of drift.stages) {
    const container = containerByPath.get(stage.container)
    if (!container) continue
    stageOf.set(container.id, stage)
    const rep = representative(m, container)
    if (rep !== null) addNode(rep)
  }
  for (const cycle of drift.cycles) cycle.path.forEach((path) => addNode(idOf(path)))

  const drawn = new Map<string, ReviewEdge>()
  for (const edge of priority) {
    if (drawn.has(edge.key)) continue
    const fresh = Number(!nodes.has(edge.from)) + Number(!nodes.has(edge.to) && edge.from !== edge.to)
    const cost = fresh + 1 + (edge.tag === 'same' ? 0 : BADGE_MARKS)
    if (marks + cost > MARK_BUDGET) continue
    addNode(edge.from)
    addNode(edge.to)
    drawn.set(edge.key, edge)
    marks += cost - fresh
  }
  const omitted = all.filter((e) => !drawn.has(e.key)).length

  for (const id of nodes.keys()) {
    if (id < 0) continue
    for (const to of m.out(id)) {
      if (marks >= MARK_BUDGET) break
      const key = edgeKey(id, to)
      const head = m.edge(id, to)
      if (!head || !nodes.has(to) || drawn.has(key) || changed.has(key)) continue
      drawn.set(key, { key, from: id, to, kind: head.kind, tag: 'same', breaks: [] })
      marks++
    }
  }

  const files = new Map<number, FileRow>()
  for (const edge of all) {
    const importer = nodes.get(edge.from)
    if (importer) importer.isImporter = true
    const row = files.get(edge.from) ?? {
      id: edge.from,
      path: pathOf(edge.from),
      isGhost: edge.from < 0,
      added: 0,
      removed: 0,
      worst: null,
    }
    if (edge.tag === 'added') {
      row.added++
      const sev = worstBreak(edge.breaks)
      if (sev && isWorse(sev, row.worst)) row.worst = sev
    } else {
      row.removed++
    }
    files.set(edge.from, row)
  }

  const importItem = (edge: ReviewEdge): ChangeItem => ({
    kind: 'import',
    edge,
    from: pathOf(edge.from),
    to: pathOf(edge.to),
  })
  const items: ChangeItem[] = [
    ...breaking.map(importItem),
    ...drift.cycles.map((cycle): ChangeItem => ({ kind: 'cycle', cycle })),
    ...drift.stages.map(
      (stage): ChangeItem => ({ kind: 'stage', stage, container: containerByPath.get(stage.container)?.id ?? null }),
    ),
    ...fixed.map(importItem),
  ]

  const rank = (r: FileRow): number => (r.worst ? SEVERITY_RANK[r.worst] : 0)
  return {
    nodes,
    edges: [...drawn.values()],
    files: [...files.values()].sort((a, b) => rank(b) - rank(a) || a.path.localeCompare(b.path)),
    items,
    stageOf,
    counts: {
      added: all.filter((e) => e.tag === 'added').length,
      removed: all.filter((e) => e.tag === 'removed').length,
      breaking: breaking.length,
      cycles: drift.cycles.length,
      stages: drift.stages.length,
    },
    omitted,
  }
}
