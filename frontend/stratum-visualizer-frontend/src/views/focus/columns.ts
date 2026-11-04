import { SCENE_MARK_BUDGET } from '../../render/budget'
import { EdgeKindLabel } from '../../render/encoding'
import { Group, Model, worst } from '../../render/model'
import { Bounds } from '../../render/viewport'
import { Violation } from '../../types'
import { entryOf } from './paint'

export const COLUMN = {
  width: 250,
  row: 18,
  groupHead: 24,
  groupGap: 8,
  layerHead: 18,
  layerGap: 8,
  maxHeight: 880,
  subGap: 34,
  hopGap: 100,
  openLimit: 8,
  titleGap: 24,
} as const

export const CARD = { w: 270, h: 124 } as const

const NEAR_X = CARD.w / 2 + 110
const MIN_HALF_HEIGHT = 80
const OPEN_GROUP_PADDING = 4

export type Side = -2 | -1 | 1 | 2
export type Hops = 1 | 2

export interface ColumnRow {
  id: number
  y: number
  edgeFindings: readonly Violation[]
}

export interface ColumnGroup {
  key: string
  group: Group
  ids: readonly number[]
  isOpen: boolean
  x: number
  y: number
  h: number
  rows: readonly ColumnRow[]
  edgeFindings: readonly Violation[]
}

export interface LayerHead {
  layer: number
  x: number
  y: number
  isContinued: boolean
}

export interface Column {
  side: Side
  groups: readonly ColumnGroup[]
  heads: readonly LayerHead[]
  count: number
  subColumns: number
  minX: number
  maxX: number
  top: number
  height: number
}

export interface Bundle {
  key: string
  x1: number
  y1: number
  x2: number
  y2: number
  pairs: readonly (readonly [number, number])[]
  kind: EdgeKindLabel
  findings: readonly Violation[]
  touches: ReadonlySet<number>
}

export interface BudgetFold {
  collapsed: number
  hidden: number
}

export interface ColumnsLayout {
  focus: number
  columns: readonly Column[]
  bundles: readonly Bundle[]
  card: Bounds
  bounds: Bounds
  fold: BudgetFold
}

export type Expanded = ReadonlyMap<string, boolean>

export const SHOW_HIDDEN = '*'

interface DrawnEdge {
  from: number
  to: number
  side: Side
  farEnd: number
  nearSide: Side | 0
}

interface Anchor {
  key: string
  xl: number
  xr: number
  y: number
}

type FindingsAt = (side: Side, id: number) => readonly Violation[]

const KIND_RANK: Record<EdgeKindLabel, number> = { static: 0, di: 1, runtime: 2 }
const NONE: readonly never[] = []

const unique = (ids: Iterable<number>, exclude: ReadonlySet<number>): number[] =>
  [...new Set(ids)].filter((id) => !exclude.has(id))

type GroupDraft = Omit<ColumnGroup, 'x' | 'y' | 'rows'>

interface SubColumn {
  groups: (GroupDraft & { y: number })[]
  heads: Omit<LayerHead, 'x'>[]
  y: number
}

const groupHeight = (count: number, isOpen: boolean): number =>
  COLUMN.groupHead + (isOpen ? count * COLUMN.row + OPEN_GROUP_PADDING : 0)

const buildGroups = (
  m: Model,
  ids: readonly number[],
  side: Side,
  expanded: Expanded,
  hidden: ReadonlySet<string>,
  findingsAt: FindingsAt,
): GroupDraft[] => {
  const byKey = new Map<string, { group: Group; ids: number[] }>()
  for (const id of ids) {
    const group = m.groupOf(id)
    const entry = byKey.get(group.key)
    if (entry) {
      entry.ids.push(id)
    } else {
      byKey.set(group.key, { group, ids: [id] })
    }
  }
  const openLimit = Math.abs(side) === 1 ? COLUMN.openLimit : 0
  return [...byKey.values()]
    .sort(
      (a, b) =>
        a.group.layer - b.group.layer || b.ids.length - a.ids.length || a.group.key.localeCompare(b.group.key),
    )
    .map(({ group, ids: members }) => {
      const key = `${side}:${group.key}`
      const isOpen = expanded.get(key) ?? members.length <= openLimit
      const sorted = members.slice().sort((a, b) => m.label(a).localeCompare(m.label(b)) || a - b)
      return {
        key,
        group,
        ids: sorted,
        isOpen,
        h: groupHeight(sorted.length, isOpen),
        edgeFindings: sorted.flatMap((id) => findingsAt(side, id)),
      }
    })
    .filter((g) => !hidden.has(g.key))
}

const stackedHeight = (groups: readonly GroupDraft[]): number =>
  groups.reduce((sum, g, i) => {
    const isNewLayer = i === 0 || groups[i - 1].group.layer !== g.group.layer
    return sum + g.h + COLUMN.groupGap + (isNewLayer ? COLUMN.layerHead + COLUMN.layerGap : 0)
  }, 0)

const splitIntoSubColumns = (groups: readonly GroupDraft[]): SubColumn[] => {
  const total = stackedHeight(groups)
  const count = Math.max(1, Math.ceil(total / COLUMN.maxHeight))
  const target = total / count
  let current: SubColumn = { groups: [], heads: [], y: 0 }
  const subs = [current]
  let lastLayer: number | null = null
  for (const g of groups) {
    const next = current.y + g.h + COLUMN.layerHead + COLUMN.layerGap
    const isBalanced = current.y + g.h > target && subs.length < count
    const isFull = current.y > 0 && (isBalanced || next > COLUMN.maxHeight)
    if (isFull) {
      current = { groups: [], heads: [], y: 0 }
      subs.push(current)
    }
    if (current.groups.length === 0 || g.group.layer !== lastLayer) {
      if (current.y > 0) current.y += COLUMN.layerGap
      current.heads.push({ layer: g.group.layer, y: current.y, isContinued: g.group.layer === lastLayer })
      current.y += COLUMN.layerHead
      lastLayer = g.group.layer
    }
    current.groups.push({ ...g, y: current.y })
    current.y += g.h + COLUMN.groupGap
  }
  return subs
}

const layoutColumn = (
  m: Model,
  ids: readonly number[],
  side: Side,
  expanded: Expanded,
  hidden: ReadonlySet<string>,
  nearX: number,
  findingsAt: FindingsAt,
): Column => {
  const subs = splitIntoSubColumns(buildGroups(m, ids, side, expanded, hidden, findingsAt))
  const dir = side > 0 ? 1 : -1
  const groups: ColumnGroup[] = []
  const heads: LayerHead[] = []
  subs.forEach((sub, i) => {
    const step = i * (COLUMN.width + COLUMN.subGap)
    const x = dir > 0 ? nearX + step : nearX - COLUMN.width - step
    const offset = -sub.y / 2
    for (const g of sub.groups) {
      const y = g.y + offset
      const rows = g.isOpen
        ? g.ids.map((id, r) => ({
            id,
            y: y + COLUMN.groupHead + r * COLUMN.row + COLUMN.row / 2,
            edgeFindings: findingsAt(side, id),
          }))
        : []
      groups.push({ ...g, x, y, rows })
    }
    for (const h of sub.heads) heads.push({ ...h, x, y: h.y + offset })
  })
  const xs = groups.map((g) => g.x)
  const minX = xs.length ? Math.min(...xs) : dir > 0 ? nearX : nearX - COLUMN.width
  const maxX = (xs.length ? Math.max(...xs) : minX) + COLUMN.width
  const height = Math.max(0, ...subs.map((s) => s.y))
  return { side, groups, heads, count: ids.length, subColumns: subs.length, minX, maxX, top: -height / 2, height }
}

const farX = (c: Column): number => (c.side > 0 ? c.maxX : c.minX)

const drawnEdges = (m: Model, focus: number, sets: Record<Side, readonly number[]>): DrawnEdge[] => {
  const edges: DrawnEdge[] = [
    ...sets[1].map((id): DrawnEdge => ({ from: focus, to: id, side: 1, farEnd: id, nearSide: 0 })),
    ...sets[-1].map((id): DrawnEdge => ({ from: id, to: focus, side: -1, farEnd: id, nearSide: 0 })),
  ]
  const out2 = new Set(sets[2])
  const in2 = new Set(sets[-2])
  for (const s of sets[1]) {
    for (const t of m.out(s)) {
      if (out2.has(t)) edges.push({ from: s, to: t, side: 2, farEnd: t, nearSide: 1 })
    }
  }
  for (const t of sets[-1]) {
    for (const s of m.inn(t)) {
      if (in2.has(s)) edges.push({ from: s, to: t, side: -2, farEnd: s, nearSide: -1 })
    }
  }
  return edges
}

const findingsIndex = (m: Model, edges: readonly DrawnEdge[]): FindingsAt => {
  const byFarEnd = new Map<string, Violation[]>()
  for (const e of edges) {
    const found = m.findingsOnEdge(e.from, e.to)
    if (!found.length) continue
    const key = `${e.side}:${e.farEnd}`
    const list = byFarEnd.get(key)
    if (list) {
      list.push(...found)
    } else {
      byFarEnd.set(key, [...found])
    }
  }
  return (side, id) => byFarEnd.get(`${side}:${id}`) ?? NONE
}

interface Anchors {
  visible: Map<string, Anchor>
  groupHeads: Map<string, Anchor>
}

const anchorsOf = (columns: readonly Column[]): Anchors => {
  const visible = new Map<string, Anchor>()
  const groupHeads = new Map<string, Anchor>()
  for (const c of columns) {
    for (const g of c.groups) {
      const box = { xl: g.x, xr: g.x + COLUMN.width }
      const head: Anchor = { ...box, key: g.key, y: g.y + COLUMN.groupHead / 2 }
      for (const id of g.ids) groupHeads.set(`${c.side}:${id}`, head)
      if (g.isOpen) {
        for (const r of g.rows) visible.set(`${c.side}:${r.id}`, { ...box, key: `${c.side}:${r.id}`, y: r.y })
        continue
      }
      for (const id of g.ids) visible.set(`${c.side}:${id}`, head)
    }
  }
  return { visible, groupHeads }
}

const hopOneEndOfHopTwo = (anchors: Anchors, expanded: Expanded, key: string): Anchor | undefined => {
  const head = anchors.groupHeads.get(key)
  if (!head) return undefined
  const isExpandedByUser = expanded.get(head.key) === true
  return isExpandedByUser ? anchors.visible.get(key) : head
}

const FOCUS_LEFT: Anchor = { key: 'F', xl: -CARD.w / 2, xr: -CARD.w / 2, y: 0 }
const FOCUS_RIGHT: Anchor = { key: 'F', xl: CARD.w / 2, xr: CARD.w / 2, y: 0 }

interface BundleDraft extends Bundle {
  pairs: [number, number][]
  findings: Violation[]
  touches: Set<number>
}

const bundleEdges = (m: Model, edges: readonly DrawnEdge[], anchors: Anchors, expanded: Expanded): Bundle[] => {
  const byKey = new Map<string, BundleDraft>()
  for (const e of edges) {
    const far = anchors.visible.get(`${e.side}:${e.farEnd}`)
    const nearId = e.side > 0 ? e.from : e.to
    const focusAnchor = e.side > 0 ? FOCUS_RIGHT : FOCUS_LEFT
    const near =
      e.nearSide === 0 ? focusAnchor : hopOneEndOfHopTwo(anchors, expanded, `${e.nearSide}:${nearId}`)
    if (!far || !near) continue
    const [a, b] = e.side > 0 ? [near, far] : [far, near]
    const key = `${a.key}|${b.key}`
    let draft = byKey.get(key)
    if (!draft) {
      draft = { key, x1: a.xr, y1: a.y, x2: b.xl, y2: b.y, pairs: [], kind: 'static', findings: [], touches: new Set() }
      byKey.set(key, draft)
    }
    draft.pairs.push([e.from, e.to])
    draft.findings.push(...m.findingsOnEdge(e.from, e.to))
    draft.touches.add(e.from)
    draft.touches.add(e.to)
    const kind = (m.edge(e.from, e.to)?.kind ?? 'static') as EdgeKindLabel
    if (KIND_RANK[kind] > KIND_RANK[draft.kind]) draft.kind = kind
  }
  return [...byKey.values()]
}

const FOCUS_CARD_MARKS = 13
const ROW_MARKS = 2
const CLOSED_GROUP_MARKS = 6

const hasSeverity = (findings: readonly Violation[]): boolean => worst(findings) !== null

const marksOf = (m: Model, focus: number, columns: readonly Column[], bundles: readonly Bundle[]): number => {
  const territory = m.territoryOf(focus)
  let marks = FOCUS_CARD_MARKS
  for (const b of bundles) marks += 2 + Number(b.pairs.length > 1)
  for (const c of columns) {
    const allowed = c.side > 0 ? territory.may_import_layers : territory.imported_by_layers
    marks += 1
    for (const h of c.heads) marks += 2 + Number(Math.abs(c.side) === 1 && !allowed.includes(h.layer))
    for (const g of c.groups) {
      marks += 4 + Number(g.edgeFindings.length > 0)
      if (!g.isOpen) marks += Number(hasSeverity(g.ids.flatMap((id) => m.findingsOf(id))))
      for (const r of g.rows) {
        marks += ROW_MARKS + Number(entryOf(m, r.id).length > 0) + Number(hasSeverity(r.edgeFindings))
        marks += Number(hasSeverity(m.findingsOf(r.id))) + Number(m.isSharedByChosenFeatures(r.id))
      }
    }
  }
  return marks
}

interface Placed {
  columns: Column[]
  bundles: Bundle[]
  marks: number
}

type Place = (open: Expanded, hidden: ReadonlySet<string>) => Placed

const foldToBudget = (place: Place, expanded: Expanded): { placed: Placed; fold: BudgetFold } => {
  const open = new Map(expanded)
  const hidden = new Set<string>()
  const fold: BudgetFold = { collapsed: 0, hidden: 0 }
  let placed = place(open, hidden)
  const isDefault = (g: ColumnGroup) => !expanded.has(g.key)
  const shrink = (candidates: readonly ColumnGroup[], saving: (g: ColumnGroup) => number, apply: (g: ColumnGroup) => void) => {
    let saved = 0
    for (const g of candidates) {
      if (saved >= placed.marks - SCENE_MARK_BUDGET) break
      apply(g)
      saved += saving(g)
    }
    placed = place(open, hidden)
  }
  while (placed.marks > SCENE_MARK_BUDGET) {
    const defaultOpen = placed.columns
      .flatMap((c) => c.groups)
      .filter((g) => g.isOpen && isDefault(g))
      .sort((a, b) => b.ids.length - a.ids.length || a.key.localeCompare(b.key))
    if (!defaultOpen.length) break
    shrink(
      defaultOpen,
      (g) => g.rows.length * ROW_MARKS,
      (g) => {
        open.set(g.key, false)
        fold.collapsed++
      },
    )
  }
  while (placed.marks > SCENE_MARK_BUDGET && expanded.get(SHOW_HIDDEN) !== true) {
    const shown = placed.columns
      .flatMap((c) => c.groups.filter(isDefault).map((g) => ({ g, hop: Math.abs(c.side) })))
      .sort((a, b) => b.hop - a.hop || a.g.ids.length - b.g.ids.length || a.g.key.localeCompare(b.g.key))
      .map(({ g }) => g)
    if (!shown.length) break
    shrink(
      shown,
      (g) => CLOSED_GROUP_MARKS + g.rows.length * ROW_MARKS,
      (g) => {
        hidden.add(g.key)
        fold.hidden++
      },
    )
  }
  return { placed, fold }
}

/**
 * `expanded` is keyed by `ColumnGroup.key` and overrides the default open state of that group; `SHOW_HIDDEN: true`
 * brings back the groups the mark budget left out. To fit `SCENE_MARK_BUDGET`, default-open groups collapse, largest
 * first, then the smallest groups (2nd hop first) leave the scene; a group named in `expanded` never folds.
 */
export const layoutColumns = (m: Model, focus: number, hops: Hops, expanded: Expanded): ColumnsLayout => {
  const out1 = unique(m.out(focus), new Set([focus]))
  const in1 = unique(m.inn(focus), new Set([focus]))
  const seen = new Set([focus, ...out1, ...in1])
  const out2 = hops > 1 ? unique(out1.flatMap(m.out), seen) : []
  const in2 = hops > 1 ? unique(in1.flatMap(m.inn), new Set([...seen, ...out2])) : []
  const edges = drawnEdges(m, focus, { 1: out1, [-1]: in1, 2: out2, [-2]: in2 })
  const findingsAt = findingsIndex(m, edges)

  const place: Place = (open, hidden) => {
    const left = layoutColumn(m, in1, -1, open, hidden, -NEAR_X, findingsAt)
    const right = layoutColumn(m, out1, 1, open, hidden, NEAR_X, findingsAt)
    const columns = [left, right]
    if (hops > 1) {
      columns.push(layoutColumn(m, in2, -2, open, hidden, farX(left) - COLUMN.hopGap, findingsAt))
      columns.push(layoutColumn(m, out2, 2, open, hidden, farX(right) + COLUMN.hopGap, findingsAt))
    }
    const bundles = bundleEdges(m, edges, anchorsOf(columns), open)
    return { columns, bundles, marks: marksOf(m, focus, columns, bundles) }
  }
  const { placed, fold } = foldToBudget(place, expanded)
  const { columns, bundles } = placed

  const card = { x: -CARD.w / 2, y: -CARD.h / 2, w: CARD.w, h: CARD.h }
  const minY = Math.min(-MIN_HALF_HEIGHT, card.y, ...columns.map((c) => c.top - COLUMN.titleGap))
  const maxY = Math.max(MIN_HALF_HEIGHT, card.y + card.h, ...columns.map((c) => c.top + c.height))
  const minX = Math.min(card.x, ...columns.map((c) => c.minX))
  const maxX = Math.max(card.x + card.w, ...columns.map((c) => c.maxX))
  return { focus, columns, bundles, card, bounds: { x: minX, y: minY, w: maxX - minX, h: maxY - minY }, fold }
}
