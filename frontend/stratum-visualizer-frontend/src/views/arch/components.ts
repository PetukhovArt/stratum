import { SCENE_MARK_BUDGET } from '../../render/budget'
import { edgeKey, Model, Unit } from '../../render/model'
import { Bounds } from '../../render/viewport'
import { curveBetween, Point, Rect } from '../map/layout'

export const ROOT_SEGMENT = ''

export interface Segment {
  name: string
  modules: readonly number[]
}

export interface ModuleNode extends Rect {
  id: number
  label: string
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
}

export const NODE = { w: 176, h: 18, gap: 4 } as const
export const GROUP = { pad: 10, head: 26, gap: 18, rowsPerCol: 14, maxCols: 4 } as const
const WIDTH = 1700
const MARGIN = 10

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

const boxOf = (m: Model, unit: Unit, segment: Segment, x: number, y: number): SegmentBox => {
  const n = segment.modules.length
  const cols = Math.max(1, Math.min(GROUP.maxCols, Math.ceil(n / GROUP.rowsPerCol)))
  const rows = Math.ceil(n / cols)
  const nodes = segment.modules.map((id, i) => ({
    id,
    label: splitSegment(unit.path, m.modules.get(id)?.path ?? '').rest,
    x: x + GROUP.pad + Math.floor(i / rows) * (NODE.w + NODE.gap),
    y: y + GROUP.head + (i % rows) * (NODE.h + NODE.gap),
    w: NODE.w,
    h: NODE.h,
  }))
  return {
    segment,
    nodes,
    x,
    y,
    w: GROUP.pad * 2 + cols * NODE.w + (cols - 1) * NODE.gap,
    h: GROUP.head + rows * (NODE.h + NODE.gap) - NODE.gap + GROUP.pad,
  }
}

export const layoutComponents = (m: Model, unit: Unit): ComponentsLayout => {
  const boxes: SegmentBox[] = []
  let x = 0
  let y = 0
  let rowH = 0
  for (const segment of segmentsOf(m, unit)) {
    let box = boxOf(m, unit, segment, x, y)
    if (x + box.w > WIDTH && x > 0) {
      x = 0
      y += rowH + GROUP.gap
      rowH = 0
      box = boxOf(m, unit, segment, x, y)
    }
    boxes.push(box)
    x += box.w + GROUP.gap
    rowH = Math.max(rowH, box.h)
  }
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
  const segmentLinks = [...bySegments.entries()].map(([key, l]) => ({
    key,
    from: boxes[l.from].segment.name,
    to: boxes[l.to].segment.name,
    count: l.count,
    ...curveBetween(boxes[l.from], boxes[l.to]),
  }))
  const w = Math.max(0, ...boxes.map((b) => b.x + b.w))
  const h = y + rowH
  return { boxes, nodes, imports, segmentLinks, bounds: { x: -MARGIN, y: -MARGIN, w: w + MARGIN * 2, h: h + MARGIN * 2 } }
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

/**
 * Past `SCENE_MARK_BUDGET` the module imports give way to segment arrows plus the imports of `focus`,
 * then the module labels go too.
 */
export const marksWithinBudget = (layout: ComponentsLayout, focus: number | null): ComponentsMarks => {
  const base = layout.boxes.length * BOX_MARKS + layout.nodes.size * NODE_MARKS
  const all = layout.imports
  if (base + all.length <= SCENE_MARK_BUDGET) return { imports: all, segmentLinks: [], hasLabels: true, hidden: 0 }
  const focused = focus === null ? [] : all.filter((a) => a.from === focus || a.to === focus)
  const arrows = layout.segmentLinks.length * SEGMENT_LINK_MARKS + focused.length
  return {
    imports: focused,
    segmentLinks: layout.segmentLinks,
    hasLabels: base + arrows <= SCENE_MARK_BUDGET,
    hidden: all.length - focused.length,
  }
}
