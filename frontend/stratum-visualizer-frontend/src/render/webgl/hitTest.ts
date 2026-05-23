import type { Scene, ModulePos } from '../layout'

export interface HitTestIndex {
  queryPoint(x: number, y: number): string | null
}

interface CellEntry {
  id: string
  x: number
  y: number
  w: number
  h: number
  area: number
}

const DEFAULT_CELL = 128

export const buildHitTest = (scene: Scene, cellSize = DEFAULT_CELL): HitTestIndex => {
  const cells = new Map<number, CellEntry[]>()
  const cols = Math.max(1, Math.ceil(scene.width / cellSize))

  const keyOf = (cx: number, cy: number): number => cy * cols + cx

  const insert = (id: string, p: ModulePos) => {
    const entry: CellEntry = { id, x: p.x, y: p.y, w: p.w, h: p.h, area: p.w * p.h }
    const cx0 = Math.floor(p.x / cellSize)
    const cy0 = Math.floor(p.y / cellSize)
    // ε on the right/bottom edge: a rect ending exactly on a cell boundary
    // belongs to the previous cell (query AABB is right/bottom-exclusive).
    const cx1 = Math.floor((p.x + p.w - 0.001) / cellSize)
    const cy1 = Math.floor((p.y + p.h - 0.001) / cellSize)
    for (let cy = cy0; cy <= cy1; cy++) {
      for (let cx = cx0; cx <= cx1; cx++) {
        const k = keyOf(cx, cy)
        let arr = cells.get(k)
        if (!arr) {
          arr = []
          cells.set(k, arr)
        }
        arr.push(entry)
      }
    }
  }

  for (const [id, p] of Object.entries(scene.modulePos)) insert(id, p)

  const queryPoint = (x: number, y: number): string | null => {
    const cx = Math.floor(x / cellSize)
    const cy = Math.floor(y / cellSize)
    const k = keyOf(cx, cy)
    const arr = cells.get(k)
    if (!arr) return null
    let bestId: string | null = null
    let bestArea = Infinity
    for (const e of arr) {
      if (x >= e.x && x < e.x + e.w && y >= e.y && y < e.y + e.h) {
        if (e.area < bestArea) {
          bestArea = e.area
          bestId = e.id
        }
      }
    }
    return bestId
  }

  return { queryPoint }
}
