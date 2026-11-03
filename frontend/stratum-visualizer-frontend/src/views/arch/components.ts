import { SCENE_MARK_BUDGET } from '../../render/budget'
import { disambiguate } from '../../render/labels'
import { edgeKey, Model, Unit } from '../../render/model'
import { Bounds } from '../../render/viewport'
import { curveBetween, Point, Rect } from '../map/layout'
import { GridLine, routeAround, RouteLink } from '../../render/route'

export const ROOT_SEGMENT = ''

export interface Segment {
  name: string
  modules: readonly number[]
}

export interface ModuleNode extends Rect {
  id: number
  label: string
  name: string
}

export interface SegmentBox extends Rect {
  segment: Segment
  nodes: readonly ModuleNode[]
}

export interface ImportArrow {
  from: number
  to: number
  d: string
}

export interface SegmentArrow {
  key: string
  from: string
  to: string
  count: number
  d: string
  label: Point
}

export interface ComponentsLayout {
  boxes: readonly SegmentBox[]
  nodes: ReadonlyMap<number, ModuleNode>
  imports: readonly ImportArrow[]
  segmentLinks: readonly SegmentArrow[]
  bounds: Bounds
  routing: { obstacles: readonly Rect[]; lines: { xs: readonly GridLine[]; ys: readonly GridLine[] } }
  boxOf: ReadonlyMap<number, number>
}

export const NODE = { h: 20, rowGap: 6, colGap: 16, minW: 132, maxW: 236, charPx: 6, textPad: 12 } as const
export const GROUP = { pad: 12, head: 30, gap: 36, rowsPerCol: 14, maxCols: 4, charPx: 6.6 } as const
const ASPECT = 1.5
const MIN_HEIGHT = 240
const HEIGHT_TRIES = 24
const MARGIN = GROUP.gap
const NAME_SHARE = 0.8
const HEAD_LANE = 5
const ORDER_PASSES = 2
const MAX_ROUTED = 120
const ELLIPSIS = '…'

const relativeTo = (unitPath: string, path: string): string =>
  unitPath && path.startsWith(`${unitPath}/`) ? path.slice(unitPath.length + 1) : path

const splitSegment = (unitPath: string, path: string): { segment: string; rest: string } => {
  const rel = relativeTo(unitPath, path)
  const slash = rel.indexOf('/')
  return slash < 0 ? { segment: ROOT_SEGMENT, rest: rel } : { segment: rel.slice(0, slash), rest: rel.slice(slash + 1) }
}

const byText = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0)

export const segmentsOf = (m: Model, unit: Unit): Segment[] => {
  const bySegment = new Map<string, number[]>()
  for (const id of unit.modules) {
    const path = m.modules.get(id)?.path
    if (path === undefined) continue
    const { segment } = splitSegment(unit.path, path)
    bySegment.set(segment, [...(bySegment.get(segment) ?? []), id])
  }
  const pathOf = (id: number) => m.modules.get(id)?.path ?? ''
  return [...bySegment.entries()]
    .map(([name, modules]) => ({ name, modules: modules.sort((a, b) => byText(pathOf(a), pathOf(b))) }))
    .sort((a, b) => b.modules.length - a.modules.length || byText(a.name, b.name))
}

const baseOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

const stemOf = (base: string) => {
  const dot = base.lastIndexOf('.')
  return dot > 0 ? base.slice(0, dot) : base
}

const fitName = (name: string, chars: number, sharedStems: ReadonlySet<string>): string => {
  if (name.length <= chars) return name
  if (name.includes('/')) return `${ELLIPSIS}${name.slice(-(chars - 1))}`
  const stem = stemOf(name)
  const ext = name.slice(stem.length)
  if (!sharedStems.has(stem)) return stem.length <= chars ? stem : `${stem.slice(0, chars - 1)}${ELLIPSIS}`
  return `${stem.slice(0, Math.max(1, chars - ext.length - 1))}${ELLIPSIS}${ext}`
}

interface Names {
  full: ReadonlyMap<number, string>
  width: number
  chars: number
  sharedStems: ReadonlySet<string>
}

const namesOf = (m: Model, unit: Unit, segments: readonly Segment[]): Names => {
  const items = segments.flatMap((s) =>
    s.modules.map((id) => {
      const rest = splitSegment(unit.path, m.modules.get(id)?.path ?? '').rest
      return { id, key: `${s.name}/${baseOf(rest)}`, label: baseOf(rest), path: rest }
    }),
  )
  const labels = disambiguate(items)
  const full = new Map(items.map((it, i) => [it.id, labels[i]]))
  const lengths = labels.map((l) => l.length).sort((a, b) => a - b)
  const typical = lengths[Math.min(lengths.length - 1, Math.floor(lengths.length * NAME_SHARE))] ?? 0
  const width = Math.max(NODE.minW, Math.min(NODE.maxW, typical * NODE.charPx + NODE.textPad))
  const stems = new Map<string, number>()
  for (const it of items) stems.set(stemOf(it.label), (stems.get(stemOf(it.label)) ?? 0) + 1)
  const sharedStems = new Set([...stems].filter(([, n]) => n > 1).map(([s]) => s))
  return { full, width, chars: Math.floor((width - NODE.textPad) / NODE.charPx), sharedStems }
}

const gridOf = (n: number) => {
  const cols = Math.max(1, Math.min(GROUP.maxCols, Math.ceil(n / GROUP.rowsPerCol)))
  return { cols, rows: Math.ceil(n / cols) }
}

const sizeOf = (names: Names, n: number) => {
  const { cols, rows } = gridOf(n)
  return {
    w: GROUP.pad * 2 + cols * names.width + (cols - 1) * NODE.colGap,
    h: GROUP.head + rows * (NODE.h + NODE.rowGap) - NODE.rowGap + GROUP.pad,
  }
}

const placeBox = (m: Model, unit: Unit, names: Names, segment: Segment, x: number, y: number): SegmentBox => {
  const { rows } = gridOf(segment.modules.length)
  const w = names.width
  const nodes = segment.modules.map((id, i) => ({
    id,
    label: splitSegment(unit.path, m.modules.get(id)?.path ?? '').rest,
    name: fitName(names.full.get(id) ?? '', names.chars, names.sharedStems),
    x: x + GROUP.pad + Math.floor(i / rows) * (w + NODE.colGap),
    y: y + GROUP.head + (i % rows) * (NODE.h + NODE.rowGap),
    w,
    h: NODE.h,
  }))
  return { segment, nodes, x, y, ...sizeOf(names, segment.modules.length) }
}

const stacks = (sizes: readonly { w: number; h: number }[], height: number) => {
  const at: Point[] = []
  let x = 0
  let y = 0
  let colW = 0
  let maxH = 0
  for (const s of sizes) {
    if (y + s.h > height && y > 0) {
      x += colW + GROUP.gap
      y = 0
      colW = 0
    }
    at.push({ x, y })
    maxH = Math.max(maxH, y + s.h)
    y += s.h + GROUP.gap
    colW = Math.max(colW, s.w)
  }
  return { at, w: x + colW, h: maxH }
}

const placeAll = (m: Model, unit: Unit, names: Names, segments: readonly Segment[]): { boxes: SegmentBox[]; h: number } => {
  const sizes = segments.map((s) => sizeOf(names, s.modules.length))
  const tallest = Math.max(MIN_HEIGHT, ...sizes.map((s) => s.h))
  const total = sizes.reduce((sum, s) => sum + s.h + GROUP.gap, 0)
  const misfit = (p: { w: number; h: number }) => Math.abs(Math.log(p.w / Math.max(1, p.h) / ASPECT))
  let best = stacks(sizes, tallest)
  for (let k = 1; k <= HEIGHT_TRIES; k++) {
    const next = stacks(sizes, tallest + ((total - tallest) * k) / HEIGHT_TRIES)
    if (misfit(next) < misfit(best) - 0.01) best = next
  }
  const boxes = segments.map((segment, i) => placeBox(m, unit, names, segment, best.at[i].x, best.at[i].y))
  return { boxes, h: best.h }
}

const byFlow = (m: Model, segments: readonly Segment[]): Segment[] => {
  const segOf = new Map(segments.flatMap((s, i) => s.modules.map((id): [number, number] => [id, i])))
  const net = segments.map(() => 0)
  for (const [id, i] of segOf) {
    for (const to of m.out(id)) {
      const j = segOf.get(to)
      if (j === undefined || j === i) continue
      net[i]++
      net[j]--
    }
  }
  return segments
    .map((s, i) => ({ s, net: net[i] }))
    .sort((a, b) => b.net - a.net || b.s.modules.length - a.s.modules.length || byText(a.s.name, b.s.name))
    .map((e) => e.s)
}

const SIDE = { left: 0, none: 1, right: 2 } as const

const reorderModules = (m: Model, boxes: readonly SegmentBox[]): Segment[] => {
  const at = new Map<number, { x: number; y: number; box: number }>()
  boxes.forEach((b, k) => b.nodes.forEach((n) => at.set(n.id, { x: n.x + n.w / 2, y: n.y + n.h / 2, box: k })))
  return boxes.map((b, k) => {
    const keyed = b.segment.modules.map((id, index) => {
      const own = at.get(id)!
      const others = [...m.out(id), ...m.inn(id)].flatMap((o) => {
        const p = at.get(o)
        return p && p.box !== k ? [p] : []
      })
      if (!others.length) return { id, side: SIDE.none as number, y: own.y, index }
      const pull = others.reduce((s, o) => s + (o.x < b.x ? -1 : o.x > b.x + b.w ? 1 : 0), 0)
      const side = pull < 0 ? SIDE.left : pull > 0 ? SIDE.right : SIDE.none
      return { id, side, y: others.reduce((s, o) => s + o.y, 0) / others.length, index }
    })
    keyed.sort((p, q) => p.side - q.side || p.y - q.y || p.index - q.index)
    return { name: b.segment.name, modules: keyed.map((e) => e.id) }
  })
}

const linesOf = (boxes: readonly SegmentBox[], cellW: number): { xs: GridLine[]; ys: GridLine[] } => {
  const xs: GridLine[] = []
  const ys: GridLine[] = []
  const outer = { cost: 1, room: GROUP.gap / 2 - 3 }
  const inner = { cost: 1.2, room: GROUP.pad / 2 - 2 }
  for (const b of boxes) {
    const { cols, rows } = gridOf(b.nodes.length)
    xs.push({ at: b.x - GROUP.gap / 2, ...outer }, { at: b.x + b.w + GROUP.gap / 2, ...outer })
    xs.push({ at: b.x + GROUP.pad / 2, ...inner }, { at: b.x + b.w - GROUP.pad / 2, ...inner })
    for (let k = 1; k < cols; k++) {
      xs.push({ at: b.x + GROUP.pad + k * (cellW + NODE.colGap) - NODE.colGap / 2, cost: 1.2, room: NODE.colGap / 2 - 3 })
    }
    ys.push({ at: b.y - GROUP.gap / 2, ...outer }, { at: b.y + b.h + GROUP.gap / 2, ...outer })
    ys.push({ at: b.y + GROUP.head - HEAD_LANE, cost: 1.3, room: 1.5 }, { at: b.y + b.h - GROUP.pad / 2, ...inner })
    for (let k = 1; k < rows; k++) {
      ys.push({ at: b.y + GROUP.head + k * (NODE.h + NODE.rowGap) - NODE.rowGap / 2, cost: 3, room: 1 })
    }
  }
  return { xs, ys }
}

const headOf = (b: SegmentBox): Rect => {
  const chars = (b.segment.name || 'root').length + 3 + String(b.segment.modules.length).length
  return { x: b.x + 6, y: b.y + 4, w: chars * GROUP.charPx + 8, h: GROUP.head - HEAD_LANE - 6 }
}

const FOLDER_TRACK_PX = 5

const linkFolders = (
  boxes: readonly SegmentBox[],
  links: ReadonlyMap<string, { from: number; to: number; count: number }>,
): SegmentArrow[] => {
  const xs: GridLine[] = []
  const ys: GridLine[] = []
  const lane = { cost: 1, room: GROUP.gap / 2 - 3 }
  for (const b of boxes) {
    xs.push({ at: b.x - GROUP.gap / 2, ...lane }, { at: b.x + b.w + GROUP.gap / 2, ...lane })
    ys.push({ at: b.y - GROUP.gap / 2, ...lane }, { at: b.y + b.h + GROUP.gap / 2, ...lane })
  }
  const reqs = [...links.entries()]
    .sort((p, q) => q[1].count - p[1].count || byText(p[0], q[0]))
    .map(([key, l]): RouteLink => ({ key, from: boxes[l.from], to: boxes[l.to] }))
  const routes = routeAround(reqs, { obstacles: boxes, lines: { xs, ys }, track: FOLDER_TRACK_PX })
  return [...links.entries()].map(([key, l]) => ({
    key,
    from: boxes[l.from].segment.name,
    to: boxes[l.to].segment.name,
    count: l.count,
    ...(routes.get(key) ?? curveBetween(boxes[l.from], boxes[l.to])),
  }))
}

export const layoutComponents = (m: Model, unit: Unit): ComponentsLayout => {
  let segments = byFlow(m, segmentsOf(m, unit))
  const names = namesOf(m, unit, segments)
  let placed = placeAll(m, unit, names, segments)
  for (let pass = 0; pass < ORDER_PASSES; pass++) {
    segments = reorderModules(m, placed.boxes)
    placed = placeAll(m, unit, names, segments)
  }
  const { boxes } = placed
  const nodes = new Map(boxes.flatMap((b) => b.nodes.map((n): [number, ModuleNode] => [n.id, n])))
  const boxIndexOf = new Map(boxes.flatMap((b, i) => b.nodes.map((n): [number, number] => [n.id, i])))

  const imports: ImportArrow[] = []
  const bySegments = new Map<string, { from: number; to: number; count: number }>()
  for (const [from, a] of nodes) {
    for (const to of m.out(from)) {
      const b = nodes.get(to)
      if (!b) continue
      imports.push({ from, to, d: curveBetween(a, b).d })
      const fromBox = boxIndexOf.get(from)!
      const toBox = boxIndexOf.get(to)!
      if (fromBox === toBox) continue
      const key = edgeKey(fromBox, toBox)
      const hit = bySegments.get(key)
      if (hit) hit.count++
      else bySegments.set(key, { from: fromBox, to: toBox, count: 1 })
    }
  }
  const segmentLinks = linkFolders(boxes, bySegments)
  const { xs, ys } = linesOf(boxes, names.width)
  const routing = { obstacles: [...nodes.values(), ...boxes.map(headOf)], lines: { xs, ys } }
  const w = Math.max(0, ...boxes.map((b) => b.x + b.w))
  return {
    boxes,
    nodes,
    imports,
    segmentLinks,
    routing,
    boxOf: boxIndexOf,
    bounds: { x: -MARGIN, y: -MARGIN, w: w + MARGIN * 2, h: placed.h + MARGIN * 2 },
  }
}

export interface ComponentsMarks {
  imports: readonly ImportArrow[]
  segmentLinks: readonly SegmentArrow[]
  hasLabels: boolean
  hidden: number
}

const BOX_MARKS = 2
const NODE_MARKS = 2
const SEGMENT_LINK_MARKS = 2

const IMPORT_SIDES = ['left', 'right'] as const

const routeRequests = (layout: ComponentsLayout, shown: readonly ImportArrow[], focus: number | null): RouteLink[] => {
  const centre = (id: number) => {
    const n = layout.nodes.get(id)!
    return { x: n.x + n.w / 2, y: n.y + n.h / 2 }
  }
  const reqs = shown.map((a) => {
    const isHub = focus !== null && (a.from === focus || a.to === focus)
    const other = a.from === focus ? a.to : a.from
    const bundle = isHub ? `${a.from === focus ? 'out' : 'in'}:${layout.boxOf.get(other)}` : `pair:${layout.boxOf.get(a.from)}>${layout.boxOf.get(a.to)}`
    const [p, q] = [centre(a.from), centre(a.to)]
    const len = Math.abs(p.x - q.x) + Math.abs(p.y - q.y)
    const joinAt = !isHub ? undefined : a.from === focus ? ('from' as const) : ('to' as const)
    return { key: `${a.from}>${a.to}`, from: a.from, to: a.to, bundle, joinAt, len }
  })
  const sizes = new Map<string, number>()
  for (const r of reqs) sizes.set(r.bundle, (sizes.get(r.bundle) ?? 0) + 1)
  return reqs
    .sort(
      (p, q) =>
        Number(q.joinAt !== undefined) - Number(p.joinAt !== undefined) ||
        sizes.get(q.bundle)! - sizes.get(p.bundle)! ||
        byText(p.bundle, q.bundle) ||
        q.len - p.len,
    )
    .slice(0, MAX_ROUTED)
    .map((r): RouteLink => ({
      key: r.key,
      from: layout.nodes.get(r.from)!,
      to: layout.nodes.get(r.to)!,
      bundle: r.bundle,
      joinAt: r.joinAt,
      sides: IMPORT_SIDES,
    }))
}

/**
 * One arrow per import turns a segment of a few dozen modules into a hairball, so segments join by one counted
 * arrow each, and only the imports of `focus` and the imports `isFlagged` marks (findings) are drawn one by one,
 * routed along the gutters between the cells. Past `SCENE_MARK_BUDGET` the module labels go too.
 */
export const componentMarks = (
  layout: ComponentsLayout,
  focus: number | null,
  isFlagged: (a: ImportArrow) => boolean,
): ComponentsMarks => {
  const base = layout.boxes.length * BOX_MARKS + layout.nodes.size * NODE_MARKS
  const shown = layout.imports.filter((a) => a.from === focus || a.to === focus || isFlagged(a))
  const routes = routeAround(routeRequests(layout, shown, focus), layout.routing)
  const arrows = layout.segmentLinks.length * SEGMENT_LINK_MARKS + shown.length
  return {
    imports: shown.map((a) => ({ ...a, d: routes.get(`${a.from}>${a.to}`)?.d ?? a.d })),
    segmentLinks: layout.segmentLinks,
    hasLabels: base + arrows <= SCENE_MARK_BUDGET,
    hidden: layout.imports.length - shown.length,
  }
}
