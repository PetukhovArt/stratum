import { describe, expect, it } from 'vitest'
import { GridLine, Rect, routeAround } from './route'

const CELL = { w: 100, h: 20 }
const PITCH = { x: 120, y: 24 }

const cellsOf = (cols: number, rows: number): Rect[] =>
  Array.from({ length: cols * rows }, (_, i) => ({ x: Math.floor(i / rows) * PITCH.x, y: (i % rows) * PITCH.y, ...CELL }))

const linesOf = (count: number, pitch: number, size: number): GridLine[] =>
  Array.from({ length: count + 1 }, (_, k) => ({ at: k * pitch - (pitch - size) / 2, cost: 1, room: 20 }))

const crosses = (a: { x: number; y: number }, b: { x: number; y: number }, r: Rect): boolean => {
  const [x0, x1] = [Math.min(a.x, b.x), Math.max(a.x, b.x)]
  const [y0, y1] = [Math.min(a.y, b.y), Math.max(a.y, b.y)]
  return x0 < r.x + r.w && x1 > r.x && y0 < r.y + r.h && y1 > r.y
}

describe('orthogonal routes', () => {
  it.each([
    ['given gutters', { xs: linesOf(4, PITCH.x, CELL.w), ys: linesOf(5, PITCH.y, CELL.h) }],
    ['derived gutters', undefined],
  ])('with %s run around every rect but their own two ends', (_, lines) => {
    const cells = cellsOf(4, 5)
    const links = cells
      .map((from, i) => ({ key: `${i}`, from, to: cells[7], bundle: `in:${i}`, joinAt: 'to' as const }))
      .filter((_, i) => i !== 7)
    const routes = routeAround(links, { obstacles: cells, lines })
    expect(routes.size).toBe(links.length)
    for (const l of links) {
      const pts = routes.get(l.key)!.points
      for (const cell of cells) {
        if (cell === l.from || cell === l.to) continue
        expect(pts.slice(1).some((p, i) => crosses(pts[i], p, cell))).toBe(false)
      }
    }
  })
})
