import { Bounds } from '../../render/viewport'
import { SeverityLabel } from '../../render/encoding'
import { AggregateEdge, Model, sccs, worst } from '../../render/model'
import { Violation } from '../../types'

export type MatrixLevel = 'layer' | 'unit'

const CROSS_LAYER_RULE = 'stratum/no-cross-layer-import'
const SIBLING_RULE = 'stratum/sibling-isolation'
const BREAK_RULES: ReadonlySet<string> = new Set([CROSS_LAYER_RULE, SIBLING_RULE])

export const GEOMETRY = {
  layer: { cell: 46, labelWidth: 130, headerHeight: 110 },
  unit: { cell: 9, labelWidth: 150, headerHeight: 150 },
} as const satisfies Record<MatrixLevel, { cell: number; labelWidth: number; headerHeight: number }>

export interface MatrixCell {
  key: string
  row: number
  col: number
  /** Layer id or unit (container) id of the importing row. */
  from: number
  /** Layer id or unit (container) id of the imported column. */
  to: number
  count: number
  rules: readonly string[]
  /** Imports in the cell that carry a cross-layer or sibling finding. */
  breaks: number
  /** Worst severity of the findings that mark the cell; null when the cell is not marked. */
  mark: SeverityLabel | null
  isAbove: boolean
}

export interface CycleBlock {
  start: number
  end: number
  size: number
}

export interface LayerRun {
  layer: number
  start: number
  end: number
}

export interface Drift {
  imports: number
  breaks: number
  above: number
}

export interface Matrix {
  level: MatrixLevel
  ids: readonly number[]
  labels: readonly string[]
  layers: readonly number[]
  cells: readonly MatrixCell[]
  blocks: readonly CycleBlock[]
  runs: readonly LayerRun[]
  drift: Drift
  cell: number
  bounds: Bounds
}

const cellKey = (from: number, to: number): string => `${from}>${to}`

const compareBy =
  <T>(...keys: ((x: T) => number | string)[]) =>
  (a: T, b: T): number => {
    for (const key of keys) {
      const ka = key(a)
      const kb = key(b)
      if (ka < kb) return -1
      if (ka > kb) return 1
    }
    return 0
  }

const greedyFeedbackOrder = (
  members: readonly number[],
  edges: readonly AggregateEdge[],
  compare: (a: number, b: number) => number,
): number[] => {
  const remaining = new Set(members)
  const outW = new Map(members.map((id) => [id, 0]))
  const inW = new Map(members.map((id) => [id, 0]))
  const outEdges = new Map<number, AggregateEdge[]>(members.map((id) => [id, []]))
  const inEdges = new Map<number, AggregateEdge[]>(members.map((id) => [id, []]))
  for (const e of edges) {
    if (!remaining.has(e.from) || !remaining.has(e.to)) continue
    outW.set(e.from, outW.get(e.from)! + e.count)
    inW.set(e.to, inW.get(e.to)! + e.count)
    outEdges.get(e.from)!.push(e)
    inEdges.get(e.to)!.push(e)
  }
  const importersFirst: number[] = []
  const importedLast: number[] = []
  const place = (id: number, isSink: boolean) => {
    if (isSink) {
      importedLast.unshift(id)
    } else {
      importersFirst.push(id)
    }
    remaining.delete(id)
    for (const e of outEdges.get(id)!) inW.set(e.to, inW.get(e.to)! - e.count)
    for (const e of inEdges.get(id)!) outW.set(e.from, outW.get(e.from)! - e.count)
  }

  const sortedMembers = [...members].sort(compare)
  while (remaining.size) {
    const candidates = sortedMembers.filter((id) => remaining.has(id))
    const sink = candidates.find((id) => outW.get(id) === 0)
    if (sink !== undefined) {
      place(sink, true)
      continue
    }
    const source = candidates.find((id) => inW.get(id) === 0)
    if (source !== undefined) {
      place(source, false)
      continue
    }
    const balance = (id: number) => outW.get(id)! - inW.get(id)!
    place(
      candidates.reduce((a, b) => (balance(b) > balance(a) ? b : a)),
      false,
    )
  }
  return [...importersFirst, ...importedLast].reverse()
}

export const partitionedOrder = (
  nodes: readonly number[],
  edges: readonly AggregateEdge[],
  compare: (a: number, b: number) => number,
): { order: number[]; blocks: CycleBlock[] } => {
  const out = new Map<number, number[]>()
  for (const e of edges) {
    const list = out.get(e.from)
    if (list) {
      list.push(e.to)
    } else {
      out.set(e.from, [e.to])
    }
  }
  const components = sccs(nodes, (n) => out.get(n) ?? []).map((c) => c.slice().sort(compare))
  const componentOf = new Map<number, number>()
  components.forEach((c, i) => c.forEach((n) => componentOf.set(n, i)))

  const importers = components.map(() => new Set<number>())
  const imported = components.map(() => new Set<number>())
  for (const e of edges) {
    const a = componentOf.get(e.from)
    const b = componentOf.get(e.to)
    if (a === undefined || b === undefined || a === b) continue
    imported[a].add(b)
    importers[b].add(a)
  }

  const pending = imported.map((s) => s.size)
  const byFirstMember = (a: number, b: number) => compare(components[a][0], components[b][0])
  const ready = components.map((_, i) => i).filter((i) => pending[i] === 0)
  const order: number[] = []
  const blocks: CycleBlock[] = []
  while (ready.length) {
    ready.sort(byFirstMember)
    const members = components[ready.shift()!]
    const isCycle = members.length > 1
    if (isCycle) {
      blocks.push({ start: order.length, end: order.length + members.length - 1, size: members.length })
    }
    order.push(...(isCycle ? greedyFeedbackOrder(members, edges, compare) : members))
    for (const importer of importers[componentOf.get(members[0])!]) {
      pending[importer]--
      if (pending[importer] === 0) ready.push(importer)
    }
  }
  return { order, blocks }
}

const isBreak = (v: Violation): boolean => BREAK_RULES.has(v.rule)
const isCrossLayer = (v: Violation): boolean => v.rule === CROSS_LAYER_RULE

const layerRuns = (layers: readonly number[]): LayerRun[] => {
  const runs: LayerRun[] = []
  layers.forEach((layer, i) => {
    const last = runs[runs.length - 1]
    if (last && last.layer === layer) {
      last.end = i
    } else {
      runs.push({ layer, start: i, end: i })
    }
  })
  return runs
}

const nodesOf = (m: Model, level: MatrixLevel) => {
  const declaredRank = new Map(
    m.layers
      .slice()
      .sort(compareBy((l) => l.depends_on.length, (l) => l.id))
      .map((l, i) => [l.id, i]),
  )
  const rankOfLayer = (layer: number) => declaredRank.get(layer) ?? declaredRank.size
  if (level === 'layer') {
    const layerName = new Map(m.layers.map((l) => [l.id, l.name]))
    return {
      ids: m.layers.map((l) => l.id),
      edges: m.layerEdges,
      endpoint: (moduleId: number) => m.modules.get(moduleId)!.layer,
      label: (id: number) => layerName.get(id) ?? String(id),
      layerOf: (id: number) => id,
      compare: compareBy<number>(rankOfLayer, (id) => id),
      marks: isCrossLayer,
    }
  }
  const unitById = new Map(m.units.map((u) => [u.id, u]))
  const unit = (id: number) => unitById.get(id)!
  return {
    ids: m.units.map((u) => u.id),
    edges: m.unitEdges,
    endpoint: m.unitOf,
    label: (id: number) => unit(id).name,
    layerOf: (id: number) => unit(id).layer,
    compare: compareBy<number>(
      (id) => rankOfLayer(unit(id).layer),
      (id) => unit(id).path,
    ),
    marks: isBreak,
  }
}

export const layoutMatrix = (m: Model, level: MatrixLevel): Matrix => {
  const nodes = nodesOf(m, level)
  const { order, blocks } = partitionedOrder(nodes.ids, nodes.edges, nodes.compare)
  const index = new Map(order.map((id, i) => [id, i]))

  const breaks = new Map<string, { count: number; found: Violation[] }>()
  for (const e of m.edges) {
    const from = nodes.endpoint(e.from)
    const to = nodes.endpoint(e.to)
    if (from === to) continue
    const findings = m.findingsOnEdge(e.from, e.to)
    const isRuleBreak = findings.some(isBreak)
    const marking = findings.filter(nodes.marks)
    if (!isRuleBreak && !marking.length) continue
    const key = cellKey(from, to)
    const acc = breaks.get(key) ?? { count: 0, found: [] }
    if (isRuleBreak) acc.count++
    acc.found.push(...marking)
    breaks.set(key, acc)
  }

  const cells: MatrixCell[] = nodes.edges.map((e) => {
    const key = cellKey(e.from, e.to)
    const row = index.get(e.from)!
    const col = index.get(e.to)!
    const found = breaks.get(key)
    return {
      key,
      row,
      col,
      from: e.from,
      to: e.to,
      count: e.count,
      rules: e.rules,
      breaks: found?.count ?? 0,
      mark: found ? worst(found.found) : null,
      isAbove: col > row,
    }
  })

  const drift: Drift = { imports: 0, breaks: 0, above: 0 }
  for (const c of cells) {
    drift.imports += c.count
    drift.breaks += c.breaks
    if (c.isAbove) drift.above += c.count
  }

  const g = GEOMETRY[level]
  const span = order.length * g.cell
  const layers = order.map(nodes.layerOf)
  return {
    level,
    ids: order,
    labels: order.map(nodes.label),
    layers,
    cells,
    blocks,
    runs: layerRuns(layers),
    drift,
    cell: g.cell,
    bounds: { x: -g.labelWidth, y: -g.headerHeight, w: g.labelWidth + span, h: g.headerHeight + span },
  }
}

const matrices = new WeakMap<Model, Partial<Record<MatrixLevel, Matrix>>>()

export const matrixOf = (model: Model, level: MatrixLevel): Matrix => {
  const byLevel = matrices.get(model) ?? {}
  matrices.set(model, byLevel)
  byLevel[level] ??= layoutMatrix(model, level)
  return byLevel[level]
}

export const pairCells = (matrix: Matrix, pair: { from: number; to: number }): MatrixCell[] =>
  matrix.cells.filter((c) => matrix.layers[c.row] === pair.from && matrix.layers[c.col] === pair.to)

export const cellOf =(matrix: Matrix, from: number, to: number): MatrixCell | null =>
  matrix.cells.find((c) => c.from === from && c.to === to) ?? null
