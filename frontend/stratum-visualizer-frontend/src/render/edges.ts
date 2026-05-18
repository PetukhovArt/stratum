import { Graphics } from 'pixi.js'
import { PositionedEdge } from './layout'

const KIND_COLORS: Record<string, number> = {
  static: 0xcccccc,
  di: 0xf5a623,
  runtime: 0xbd10e0,
}

const EDGE_FROM = '__edgeFrom'
const EDGE_TO = '__edgeTo'

export interface EdgeGraphics extends Graphics {
  [EDGE_FROM]?: string
  [EDGE_TO]?: string
}

export const buildEdgeGraphics = (edge: PositionedEdge): EdgeGraphics => {
  const g: EdgeGraphics = new Graphics()
  g[EDGE_FROM] = edge.from
  g[EDGE_TO] = edge.to

  if (edge.points.length < 2) return g

  const color = KIND_COLORS[edge.kind] ?? 0xaaaaaa
  const first = edge.points[0]
  g.moveTo(first.x, first.y)
  for (let i = 1; i < edge.points.length; i++) {
    g.lineTo(edge.points[i].x, edge.points[i].y)
  }
  g.stroke({ width: 1.5, color, alpha: 0.9 })

  drawArrowhead(g, edge.points[edge.points.length - 2], edge.points[edge.points.length - 1], color)
  return g
}

const drawArrowhead = (
  g: Graphics,
  from: { x: number; y: number },
  to: { x: number; y: number },
  color: number,
): void => {
  const dx = to.x - from.x
  const dy = to.y - from.y
  const len = Math.hypot(dx, dy)
  if (len === 0) return
  const ux = dx / len
  const uy = dy / len
  const size = 8
  const baseX = to.x - ux * size
  const baseY = to.y - uy * size
  const perpX = -uy
  const perpY = ux
  g.poly([
    to.x,
    to.y,
    baseX + perpX * size * 0.5,
    baseY + perpY * size * 0.5,
    baseX - perpX * size * 0.5,
    baseY - perpY * size * 0.5,
  ])
  g.fill({ color })
}
