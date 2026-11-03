import { t } from '../../i18n'
import { ellipsizePath } from '../../render/labels'
import { Model } from '../../render/model'
import { curveBetween, Point, Rect, routeAround, RouteLink } from '../../render/route'
import { Bounds } from '../../render/viewport'
import { Fold, Folder, Review, ReviewEdge, worstBreak } from './review'

export interface Size {
  w: number
  h: number
}

export const DEFAULT_SIZE: Size = { w: 1100, h: 650 }

export const CHIP = { h: 26, padX: 9, gap: 10, rowGap: 12, nameChar: 6.9, metaChar: 6.7, metaGap: 10, dot: 10, nameChars: 34 } as const
export const LANE = { pad: 8, gapX: 16, gapY: 34, labelChar: 7.6, labelPad: 14 } as const
export const CELL = { w: 184, h: 22, gap: 12 } as const
export const OPEN = { head: 30, pad: 8 } as const

const FIT_PAD = 80
const LEGEND = { w: 330, h: 96 } as const
const WRAP_TRIES = 28
const MAX_COLS = 8
const ROUTE_BEND = 70
const FOLD_TRACK = 5
const EDGE_TRACK = 3
const BREAK_WEIGHT = 1000
const WEIGHT = { breaking: 3, changed: 2, same: 1 } as const

export interface ChipText {
  name: string
  counts: string
  breaks: string
  stage: string
  gone: string
}

export interface FolderBox extends Rect {
  folder: Folder
  text: ChipText
}

export interface LaneBox extends Rect {
  layer: number | null
  name: string
}

export interface EdgeMark {
  edge: ReviewEdge
  d: string
  mid: Point
}

export interface FoldMark {
  fold: Fold
  d: string
  mid: Point
}

export interface OverviewLayout {
  lanes: LaneBox[]
  boxes: FolderBox[]
  breaks: EdgeMark[]
  obstacles: Rect[]
  boxOf: ReadonlyMap<string, FolderBox>
  bounds: Bounds
}

interface End {
  key: string
  rect: Rect
  isCell: boolean
}

export interface FolderLayout {
  lanes: LaneBox[]
  open: FolderBox
  boxes: FolderBox[]
  nodes: Map<number, Rect>
  bounds: Bounds
  obstacles: Rect[]
  endOf: (id: number) => End | null
}

interface Item {
  key: string
  layer: number | null
  w: number
  h: number
}

interface Packed {
  lanes: LaneBox[]
  at: Map<string, Rect>
  bounds: Bounds
  labels: Rect[]
}

export const stageText = (review: Review, container: number | null, fallback?: number): string => {
  const change = container === null ? undefined : review.stageOf.get(container)
  if (change) return change.base === null ? `→ s${change.head}` : `s${change.base} → s${change.head}`
  return fallback ? `s${fallback}` : ''
}

export const chipText = (review: Review, f: Folder): ChipText => ({
  name: ellipsizePath(f.name || f.path, CHIP.nameChars),
  counts: f.added || f.removed ? `+${f.added} −${f.removed}` : '',
  breaks: f.breaks ? String(f.breaks) : '',
  stage: stageText(review, f.container),
  gone: f.ghosts ? t('mr.box.gone', { n: f.ghosts }) : '',
})

export const metaWidth = (text: ChipText): number => {
  const parts = [text.counts, text.stage, text.gone].filter(Boolean)
  const breaks = text.breaks ? CHIP.dot + text.breaks.length * CHIP.metaChar + CHIP.metaGap : 0
  return parts.reduce((w, p) => w + p.length * CHIP.metaChar + CHIP.metaGap, breaks)
}

const chipWidth = (text: ChipText): number => CHIP.padX * 2 + text.name.length * CHIP.nameChar + metaWidth(text)

const laneName = (m: Model, layer: number | null): string =>
  (layer === null ? t('mr.lane.baseOnly') : (m.layers.find((l) => l.id === layer)?.name ?? '')).toUpperCase()

const laneOrder = (m: Model, layer: number | null): number => {
  const i = layer === null ? -1 : m.layers.findIndex((l) => l.id === layer)
  return i < 0 ? m.layers.length : i
}

const pack = (m: Model, items: readonly Item[], wrap: number): Packed => {
  const byLane = new Map<number | null, Item[]>()
  for (const it of items) byLane.set(it.layer, [...(byLane.get(it.layer) ?? []), it])
  const order = [...byLane.keys()].sort((a, b) => laneOrder(m, a) - laneOrder(m, b))
  const lanes: LaneBox[] = []
  const labels: Rect[] = []
  const at = new Map<string, Rect>()
  let x = 0
  let y = 0
  let rowH = 0
  let right = 0
  for (const layer of order) {
    const name = laneName(m, layer)
    const labelW = name.length * LANE.labelChar + LANE.labelPad
    const own = byLane.get(layer)!
    const room = Math.max(wrap - labelW - LANE.pad * 2, ...own.map((it) => it.w))
    const lines: Item[][] = [[]]
    let lineW = 0
    for (const it of own) {
      const line = lines[lines.length - 1]
      const next = lineW + (line.length ? CHIP.gap : 0) + it.w
      if (line.length && next > room) {
        lines.push([it])
        lineW = it.w
      } else {
        line.push(it)
        lineW = next
      }
    }
    const widthOf = (line: readonly Item[]) => line.reduce((s, it) => s + it.w, 0) + CHIP.gap * (line.length - 1)
    const heightOf = (line: readonly Item[]) => Math.max(...line.map((it) => it.h))
    const innerW = Math.max(...lines.map(widthOf))
    const innerH = lines.reduce((s, line) => s + heightOf(line), 0) + CHIP.rowGap * (lines.length - 1)
    const w = LANE.pad * 2 + labelW + innerW
    const h = LANE.pad * 2 + innerH
    if (x > 0 && x + w > wrap) {
      x = 0
      y += rowH + LANE.gapY
      rowH = 0
    }
    lanes.push({ layer, name, x, y, w, h })
    labels.push({ x: x + LANE.pad, y: y + LANE.pad, w: labelW - LANE.labelPad / 2, h: CHIP.h })
    let ly = y + LANE.pad
    for (const line of lines) {
      let lx = x + LANE.pad + labelW
      for (const it of line) {
        at.set(it.key, { x: lx, y: ly, w: it.w, h: it.h })
        lx += it.w + CHIP.gap
      }
      ly += heightOf(line) + CHIP.rowGap
    }
    x += w + LANE.gapX
    right = Math.max(right, x - LANE.gapX)
    rowH = Math.max(rowH, h)
  }
  const margin = LANE.gapY / 2
  return { lanes, at, labels, bounds: { x: -margin, y: -margin, w: right + margin * 2, h: y + rowH + margin * 2 } }
}

const fitIn = (b: Bounds, w: number, h: number): number => Math.min((w - FIT_PAD) / b.w, (h - FIT_PAD) / b.h)

const fitOf = (b: Bounds, size: Size): number =>
  Math.max(fitIn(b, size.w - LEGEND.w, size.h), fitIn(b, size.w, size.h - LEGEND.h))

const packToFit = (m: Model, items: readonly Item[], size: Size): Packed => {
  const widest = Math.max(...items.map((it) => it.w)) + 160
  const total = items.reduce((s, it) => s + it.w + CHIP.gap + LANE.pad * 2, 0) + 160
  let best: Packed | null = null
  for (let i = 0; i <= WRAP_TRIES; i++) {
    const wrap = widest * Math.pow(Math.max(1, total / widest), i / WRAP_TRIES)
    const packed = pack(m, items, wrap)
    if (!best || fitOf(packed.bounds, size) > fitOf(best.bounds, size)) best = packed
  }
  return best!
}

const weightOf = (e: ReviewEdge): number =>
  e.breaks.length ? WEIGHT.breaking : e.tag === 'same' ? WEIGHT.same : WEIGHT.changed

const bundleOf = (from: string, to: string, e: ReviewEdge): string =>
  `${from}>${to}:${e.tag}:${worstBreak(e.breaks) ?? ''}:${e.kind}`

export const layoutOverview = (m: Model, review: Review, size: Size = DEFAULT_SIZE): OverviewLayout => {
  const texts = new Map(review.folders.map((f) => [f.key, chipText(review, f)]))
  const items = review.folders.map((f): Item => ({ key: f.key, layer: f.layer, w: chipWidth(texts.get(f.key)!), h: CHIP.h }))
  const packed = packToFit(m, items, size)
  const boxes = review.folders.map((folder): FolderBox => ({ folder, text: texts.get(folder.key)!, ...packed.at.get(folder.key)! }))
  const boxOf = new Map(boxes.map((b) => [b.folder.key, b]))
  const groupOf = (id: number) => review.nodes.get(id)!.group.key
  const crossing = review.alwaysDrawn.filter((e) => groupOf(e.from) !== groupOf(e.to))
  const links: RouteLink[] = [
    ...crossing.map((e) => ({
      key: e.key,
      from: boxOf.get(groupOf(e.from))!,
      to: boxOf.get(groupOf(e.to))!,
      weight: BREAK_WEIGHT,
      bundle: bundleOf(groupOf(e.from), groupOf(e.to), e),
    })),
  ]
  const obstacles = [...boxes, ...packed.labels]
  const routes = routeAround(links, { obstacles, bend: ROUTE_BEND, track: FOLD_TRACK })
  return {
    lanes: packed.lanes,
    boxes,
    breaks: crossing.map((edge): EdgeMark => {
      const from = boxOf.get(groupOf(edge.from))!
      const to = boxOf.get(groupOf(edge.to))!
      const r = routes.get(edge.key) ?? curveBetween(from, to)
      return { edge, d: r.d, mid: r.label }
    }),
    obstacles,
    boxOf,
    bounds: packed.bounds,
  }
}

export const routeFolds = (layout: OverviewLayout, folds: readonly Fold[]): FoldMark[] => {
  if (!folds.length) return []
  const links = folds.map(
    (f): RouteLink => ({
      key: f.key,
      from: layout.boxOf.get(f.from)!,
      to: layout.boxOf.get(f.to)!,
      weight: f.added + f.removed,
      bundle: `into:${f.to}`,
      joinAt: 'to',
    }),
  )
  const routes = routeAround(links, { obstacles: layout.obstacles, bend: ROUTE_BEND, track: FOLD_TRACK })
  return folds.map((fold): FoldMark => {
    const r = routes.get(fold.key) ?? curveBetween(layout.boxOf.get(fold.from)!, layout.boxOf.get(fold.to)!)
    return { fold, d: r.d, mid: r.label }
  })
}

export const neighboursOf = (review: Review, key: string): Set<string> => {
  const groupOf = (id: number) => review.nodes.get(id)?.group.key
  const out = new Set<string>()
  for (const e of [...review.changes, ...review.alwaysDrawn]) {
    const [from, to] = [groupOf(e.from), groupOf(e.to)]
    if (from === key && to && to !== key) out.add(to)
    if (to === key && from && from !== key) out.add(from)
  }
  return out
}

const openSize = (count: number, cols: number): { w: number; h: number } => {
  const rows = Math.ceil(count / cols)
  return {
    w: OPEN.pad * 2 + cols * CELL.w + (cols - 1) * CELL.gap,
    h: OPEN.head + rows * CELL.h + (rows - 1) * CELL.gap + OPEN.pad,
  }
}

export const layoutFolder = (m: Model, review: Review, key: string, size: Size = DEFAULT_SIZE): FolderLayout => {
  const folder = review.folderOf.get(key)!
  const near = neighboursOf(review, key)
  const others = review.folders.filter((f) => near.has(f.key))
  const texts = new Map([folder, ...others].map((f) => [f.key, chipText(review, f)]))
  const chips = others.map((f): Item => ({ key: f.key, layer: f.layer, w: chipWidth(texts.get(f.key)!), h: CHIP.h }))
  const count = folder.members.length
  const headW = chipWidth(texts.get(key)!)
  let best: { packed: Packed; cols: number } | null = null
  for (let cols = 1; cols <= Math.min(count, MAX_COLS); cols++) {
    const s = openSize(count, cols)
    const packed = packToFit(m, [...chips, { key, layer: folder.layer, w: Math.max(s.w, headW), h: s.h }], size)
    if (!best || fitOf(packed.bounds, size) > fitOf(best.packed.bounds, size)) best = { packed, cols }
  }
  const { packed, cols } = best!
  const openAt = packed.at.get(key)!
  const nodes = new Map<number, Rect>()
  folder.members.forEach((id, i) => {
    const col = i % cols
    const row = Math.floor(i / cols)
    nodes.set(id, {
      x: openAt.x + OPEN.pad + col * (CELL.w + CELL.gap),
      y: openAt.y + OPEN.head + row * (CELL.h + CELL.gap),
      w: CELL.w,
      h: CELL.h,
    })
  })
  const boxes = others.map((f): FolderBox => ({ folder: f, text: texts.get(f.key)!, ...packed.at.get(f.key)! }))
  const boxOf = new Map(boxes.map((b) => [b.folder.key, b]))
  const head: Rect = { x: openAt.x, y: openAt.y, w: openAt.w, h: OPEN.head - 4 }
  return {
    lanes: packed.lanes,
    open: { folder, text: texts.get(key)!, ...openAt },
    boxes,
    nodes,
    bounds: packed.bounds,
    obstacles: [...nodes.values(), ...boxes, ...packed.labels, head],
    endOf: (id) => {
      const cell = nodes.get(id)
      if (cell) return { key: `n${id}`, rect: cell, isCell: true }
      const group = review.nodes.get(id)?.group.key
      const box = group === undefined ? undefined : boxOf.get(group)
      return box ? { key: group!, rect: box, isCell: false } : null
    },
  }
}

export const routeFolderEdges = (layout: FolderLayout, edges: readonly ReviewEdge[]): EdgeMark[] => {
  const drawn = edges.flatMap((edge) => {
    const from = layout.endOf(edge.from)
    const to = layout.endOf(edge.to)
    return from && to && from.key !== to.key ? [{ edge, from, to }] : []
  })
  const links = drawn.map(
    ({ edge, from, to }): RouteLink => ({
      key: edge.key,
      from: from.rect,
      to: to.rect,
      weight: weightOf(edge),
      bundle: bundleOf(from.key, to.key, edge),
      joinAt: to.isCell ? (from.isCell ? undefined : 'from') : 'to',
    }),
  )
  const routes = routeAround(links, { obstacles: layout.obstacles, bend: ROUTE_BEND, track: EDGE_TRACK })
  return drawn.map(({ edge, from, to }): EdgeMark => {
    const route = routes.get(edge.key) ?? curveBetween(from.rect, to.rect)
    return { edge, d: route.d, mid: route.label }
  })
}
