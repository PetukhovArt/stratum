import { Graphics } from 'pixi.js'
import { PositionedEdge } from './layout'

const KIND_COLORS: Record<string, number> = {
  static: 0xcccccc,
  di: 0xf5a623,
  runtime: 0xbd10e0,
}

const ERROR_COLOR = 0xff5252
const WARN_COLOR = 0xf5c518

const EDGE_FROM = '__edgeFrom'
const EDGE_TO = '__edgeTo'

export interface EdgeGraphics extends Graphics {
  [EDGE_FROM]?: string
  [EDGE_TO]?: string
}

export interface EdgeGraphicsOptions {
  hasError?: boolean
  hasWarning?: boolean
}

export const buildEdgeGraphics = (
  edge: PositionedEdge,
  options: EdgeGraphicsOptions = {},
): EdgeGraphics => {
  const g: EdgeGraphics = new Graphics()
  g[EDGE_FROM] = edge.from
  g[EDGE_TO] = edge.to

  if (edge.points.length < 2) return g

  const color = options.hasError
    ? ERROR_COLOR
    : options.hasWarning
      ? WARN_COLOR
      : (KIND_COLORS[edge.kind] ?? 0xaaaaaa)
  const width = options.hasError ? 2.5 : 1.5
  const alpha = options.hasError ? 1 : 0.9
  const first = edge.points[0]
  g.moveTo(first.x, first.y)
  for (let i = 1; i < edge.points.length; i++) {
    g.lineTo(edge.points[i].x, edge.points[i].y)
  }
  g.stroke({ width, color, alpha })

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
