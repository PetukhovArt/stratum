import { describe, expect, it } from 'vitest'
import { PositionedEdge } from './layout'
import { buildEdgeGraphics } from './edges'

const edge: PositionedEdge = {
  from: 'a',
  to: 'b',
  kind: 'static',
  points: [
    { x: 0, y: 0 },
    { x: 50, y: 25 },
    { x: 100, y: 50 },
  ],
}

describe('buildEdgeGraphics', () => {
  it('attaches source/target ids', () => {
    const g = buildEdgeGraphics(edge)
    expect((g as { __edgeFrom?: string }).__edgeFrom).toBe('a')
    expect((g as { __edgeTo?: string }).__edgeTo).toBe('b')
  })

  it('handles single-segment edges', () => {
    const e: PositionedEdge = { ...edge, points: [{ x: 0, y: 0 }, { x: 10, y: 10 }] }
    expect(() => buildEdgeGraphics(e)).not.toThrow()
  })

  it('handles empty point lists', () => {
    const e: PositionedEdge = { ...edge, points: [] }
    expect(() => buildEdgeGraphics(e)).not.toThrow()
  })
})
