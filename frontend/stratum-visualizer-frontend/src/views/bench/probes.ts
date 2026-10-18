import { activeCanvas } from '../../canvas/active'
import { SEV_COLOR, SEVERITY_LABELS } from '../../render/encoding'

const DRAWN_MARKS = 'rect,path,line,circle,ellipse,text,polygon,polyline'
const MAX_CROSSING_EDGES = 900
const CROSSING_SAMPLES = 6
const SHARED_END_PX = 30
const SEGMENT_END_MARGIN = 0.02
const PAN_ZOOM_STEPS = 24
const PAN_ZOOM_PERCENTILE = 0.95
const LEAK_SAMPLES = 3
const COLOUR_PROPERTIES = ['color', 'fill', 'stroke', 'backgroundColor', 'borderLeftColor', 'borderTopColor'] as const

type Point = readonly [x: number, y: number]
type Segment = { a: Point; b: Point; edge: number }

export interface PanZoom {
  p95: number
  max: number
}

export interface ChannelAudit {
  checked: number
  leaks: number
  samples: string[]
}

export const stageSvg = (): SVGSVGElement | null => document.querySelector('.stage-region svg.cv')

export const forceLayout = (): void => {
  void document.body.offsetHeight
}

export const countMarks = (svg: SVGSVGElement | null): number =>
  svg ? [...svg.querySelectorAll(DRAWN_MARKS)].filter((el) => !el.closest('defs')).length : 0

export const boundsSize = (svg: SVGSVGElement | null): [w: number, h: number] | null => {
  const raw = svg?.querySelector('g[data-bounds]')?.getAttribute('data-bounds')
  if (!raw) return null
  const [, , w, h] = raw.split(' ').map(Number)
  return [Math.round(w), Math.round(h)]
}

export const lineEdges = (svg: SVGSVGElement | null): SVGGeometryElement[] =>
  svg ? [...svg.querySelectorAll<SVGGeometryElement>('path[data-edge]')].filter((el) => !el.closest('.dsm')) : []

const sample = (el: SVGGeometryElement): Point[] => {
  const length = el.getTotalLength()
  return Array.from({ length: CROSSING_SAMPLES + 1 }, (_, i) => {
    const p = el.getPointAtLength((length * i) / CROSSING_SAMPLES)
    return [p.x, p.y] as const
  })
}

const isNear = (p: Point, q: Point): boolean => Math.abs(p[0] - q[0]) + Math.abs(p[1] - q[1]) < SHARED_END_PX

const sharesEnd = (s: Segment, t: Segment): boolean =>
  isNear(s.a, t.a) || isNear(s.a, t.b) || isNear(s.b, t.a) || isNear(s.b, t.b)

const isInside = (u: number): boolean => u > SEGMENT_END_MARGIN && u < 1 - SEGMENT_END_MARGIN

const intersects = (s: Segment, t: Segment): boolean => {
  const [x1, y1] = s.a
  const [x2, y2] = s.b
  const [x3, y3] = t.a
  const [x4, y4] = t.b
  const d = (y4 - y3) * (x2 - x1) - (x4 - x3) * (y2 - y1)
  if (!d) return false
  const ua = ((x4 - x3) * (y1 - y3) - (y4 - y3) * (x1 - x3)) / d
  const ub = ((x2 - x1) * (y1 - y3) - (y2 - y1) * (x1 - x3)) / d
  return isInside(ua) && isInside(ub)
}

const boxesOverlap = (s: Segment, t: Segment): boolean =>
  Math.max(s.a[0], s.b[0]) >= Math.min(t.a[0], t.b[0]) &&
  Math.max(t.a[0], t.b[0]) >= Math.min(s.a[0], s.b[0]) &&
  Math.max(s.a[1], s.b[1]) >= Math.min(t.a[1], t.b[1]) &&
  Math.max(t.a[1], t.b[1]) >= Math.min(s.a[1], s.b[1])

/** Null when there are more than 900 edges: the pairwise segment test is quadratic. */
export const countCrossings = (edges: readonly SVGGeometryElement[]): number | null => {
  if (edges.length > MAX_CROSSING_EDGES) return null
  const segments: Segment[] = edges.flatMap((el, edge) => {
    const points = sample(el)
    return points.slice(1).map((b, i) => ({ a: points[i], b, edge }))
  })
  let crossings = 0
  for (let i = 0; i < segments.length; i++) {
    for (let j = i + 1; j < segments.length; j++) {
      const s = segments[i]
      const t = segments[j]
      const isCandidate = s.edge !== t.edge && boxesOverlap(s, t) && !sharesEnd(s, t)
      if (isCandidate && intersects(s, t)) crossings++
    }
  }
  return crossings
}

/** Milliseconds per viewport change: synchronous commit plus forced layout, paint excluded. */
export const measurePanZoom = (): PanZoom | null => {
  const cv = activeCanvas()
  if (!cv) return null
  const start = cv.get()
  const durations: number[] = []
  for (let i = 0; i < PAN_ZOOM_STEPS; i++) {
    const t0 = performance.now()
    cv.set({
      zoom: start.zoom * (1 + 0.15 * Math.sin(i / 4)),
      x: start.x + i * 6,
      y: start.y + Math.sin(i / 3) * 20,
    })
    forceLayout()
    durations.push(performance.now() - t0)
  }
  cv.set(start)
  durations.sort((a, b) => a - b)
  return { p95: durations[Math.floor(durations.length * PAN_ZOOM_PERCENTILE)], max: durations[durations.length - 1] }
}

export const severityColours = (): ReadonlySet<string> => {
  const probe = document.createElement('span')
  document.body.append(probe)
  const computed = SEVERITY_LABELS.map((label) => {
    probe.style.color = SEV_COLOR[label]
    return getComputedStyle(probe).color
  })
  probe.remove()
  return new Set(computed)
}

const describe = (el: Element): string => {
  const cls = el.getAttribute('class')
  const text = el.textContent?.trim().slice(0, 20)
  return `${el.tagName.toLowerCase()}${cls ? `.${cls.split(' ').join('.')}` : ''}${text ? `:${text}` : ''}`
}

export const auditChannel = (root: Element | null, colours: ReadonlySet<string>): ChannelAudit => {
  const result: ChannelAudit = { checked: 0, leaks: 0, samples: [] }
  if (!root) return result
  for (const el of root.querySelectorAll('*')) {
    const style = getComputedStyle(el)
    const isSeverityColoured = COLOUR_PROPERTIES.some((p) => colours.has(style[p]))
    if (!isSeverityColoured) continue
    result.checked++
    if (el.closest('[data-ch="sev"]')) continue
    result.leaks++
    if (result.samples.length < LEAK_SAMPLES) result.samples.push(describe(el))
  }
  return result
}
