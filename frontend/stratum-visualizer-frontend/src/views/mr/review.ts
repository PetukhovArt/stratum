import { SEVERITY_RANK, SeverityLabel } from '../../render/encoding'
import { edgeKey, Model } from '../../render/model'
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

export interface Tally {
  added: number
  removed: number
}

export interface Fold extends Tally {
  key: string
  from: string
  to: string
  worst: SeverityLabel | null
  edges: readonly ReviewEdge[]
}

export interface Review {
  nodes: ReadonlyMap<number, ReviewNode>
  changes: readonly ReviewEdge[]
  alwaysDrawn: readonly ReviewEdge[]
  folds: readonly Fold[]
  innerByGroup: ReadonlyMap<string, Tally>
  files: readonly FileRow[]
  items: readonly ChangeItem[]
  stageOf: ReadonlyMap<number, StageChange>
  counts: { added: number; removed: number; breaking: number; cycles: number; stages: number }
  /** Changed imports left out of the region to stay within the mark budget. */
  omitted: number
}

export const MARK_BUDGET = 1300
const MARKS = { node: 2, moduleEdge: 2, fold: 3 } as const

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

const inScope = (m: Model, drift: DriftReport): DriftReport => {
  const newIndex = new Map<number, number>()
  const edges = drift.edges.filter((e, i) => {
    if (m.hides(e.from) || m.hides(e.to)) return false
    newIndex.set(i, newIndex.size)
    return true
  })
  return {
    ...drift,
    edges,
    breaking: drift.breaking.flatMap((i) => newIndex.get(i) ?? []),
    cycles: drift.cycles.filter((c) => !c.members.some(m.hides) && !c.path.some(m.hides)),
    stages: drift.stages.filter((s) => !m.hides(s.container)),
  }
}

const tallyOf = (edges: readonly ReviewEdge[]): Tally => ({
  added: edges.filter((e) => e.tag === 'added').length,
  removed: edges.filter((e) => e.tag === 'removed').length,
})

const foldOf = (
  nodes: ReadonlyMap<number, ReviewNode>,
  changes: readonly ReviewEdge[],
  marked: ReadonlyMap<string, ReviewEdge>,
): { folds: Fold[]; inner: Map<string, Tally> } => {
  const between = new Map<string, ReviewEdge[]>()
  const within = new Map<string, ReviewEdge[]>()
  for (const e of changes) {
    const from = nodes.get(e.from)!.group.key
    const to = nodes.get(e.to)!.group.key
    const [byKey, key] = from === to ? [within, from] : [between, `${from}>${to}`]
    byKey.set(key, [...(byKey.get(key) ?? []), e])
  }
  const folds = [...between]
    .filter(([, edges]) => edges.some((e) => !marked.has(e.key)))
    .map(([key, edges]): Fold => {
      const [from, to] = key.split('>')
      let worst: SeverityLabel | null = null
      for (const e of edges) {
        const sev = worstBreak(e.breaks)
        if (sev && isWorse(sev, worst)) worst = sev
      }
      return { key, from, to, ...tallyOf(edges), worst, edges }
    })
  return { folds, inner: new Map([...within].map(([key, edges]) => [key, tallyOf(edges)])) }
}

const unchanged = (m: Model, review: Review, from: number, to: number): ReviewEdge | null => {
  const head = review.nodes.has(from) && review.nodes.has(to) ? m.edge(from, to) : undefined
  return head ? { key: edgeKey(from, to), from, to, kind: head.kind, tag: 'same', breaks: [] } : null
}

export const linesOfModule = (m: Model, review: Review, id: number): ReviewEdge[] => {
  const changed = review.changes.filter((e) => e.from === id || e.to === id)
  if (id < 0) return changed
  const keys = new Set(changed.map((e) => e.key))
  const pairs: [number, number][] = [...m.out(id).map((to): [number, number] => [id, to]), ...m.inn(id).map((from): [number, number] => [from, id])]
  const context = pairs
    .filter(([from, to]) => !keys.has(edgeKey(from, to)))
    .flatMap(([from, to]) => unchanged(m, review, from, to) ?? [])
  return [...changed, ...context]
}

export const edgeOf = (m: Model, review: Review, from: number, to: number): ReviewEdge | null => {
  const key = edgeKey(from, to)
  return review.changes.find((e) => e.key === key) ?? review.alwaysDrawn.find((e) => e.key === key) ?? unchanged(m, review, from, to)
}

export const toReview = (m: Model, whole: DriftReport): Review => {
  const drift = inScope(m, whole)
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
  const marked = new Map([...breaking, ...cycleEdges, ...fixed].map((e) => [e.key, e]))
  const rest = (tag: EdgeTag) => all.filter((e) => e.tag === tag && !marked.has(e.key))
  const priority = [...marked.values(), ...rest('added'), ...rest('removed')]

  const known = new Map<number, ReviewNode>()
  const nodeOf = (id: number): ReviewNode => {
    const made = known.get(id) ?? (ghostPaths.has(id) ? ghostNode(m, id, ghostPaths.get(id)!, byDepth) : headNode(m, id))
    known.set(id, made)
    return made
  }
  const nodes = new Map<number, ReviewNode>()
  let marks = 0
  const addNode = (id: number): void => {
    if (nodes.has(id)) return
    nodes.set(id, nodeOf(id))
    marks += MARKS.node
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

  const placed = new Map<string, ReviewEdge>()
  const pairs = new Set<string>()
  for (const edge of priority) {
    if (placed.has(edge.key)) continue
    const fresh = Number(!nodes.has(edge.from)) + Number(!nodes.has(edge.to) && edge.from !== edge.to)
    const from = nodeOf(edge.from).group.key
    const to = nodeOf(edge.to).group.key
    const pair = `${from}>${to}`
    const isFolded = !marked.has(edge.key) && from !== to
    const line = marked.has(edge.key) ? MARKS.moduleEdge : isFolded && !pairs.has(pair) ? MARKS.fold : 0
    const cost = fresh * MARKS.node + line
    if (marks + cost > MARK_BUDGET) continue
    addNode(edge.from)
    addNode(edge.to)
    placed.set(edge.key, edge)
    if (isFolded) pairs.add(pair)
    marks += line
  }
  const omitted = all.filter((e) => !placed.has(e.key)).length
  const changes = all.filter((e) => placed.has(e.key))
  const { folds, inner } = foldOf(nodes, changes, marked)

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
    changes,
    alwaysDrawn: [...marked.values()].filter((e) => placed.has(e.key)),
    folds,
    innerByGroup: inner,
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
