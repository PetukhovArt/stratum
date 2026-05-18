import { describe, expect, it } from 'vitest'
import { RenderGraph } from './adapt'
import { computeLayout } from './layout'

const graph: RenderGraph = {
  nodes: [
    { id: 'a', label: 'A', layer: 0, kind: 'module', parent: null },
    { id: 'b', label: 'B', layer: 1, kind: 'module', parent: null },
  ],
  edges: [{ from: 'a', to: 'b', kind: 'static' }],
  layerNames: ['domain', 'app'],
}

describe('computeLayout', () => {
  it('returns one positioned node per input node', () => {
    const out = computeLayout(graph, { rankdir: 'LR' })
    expect(out.nodes).toHaveLength(2)
    expect(out.nodes.map((n) => n.id).sort()).toEqual(['a', 'b'])
  })

  it('preserves edge endpoints with at least two points', () => {
    const out = computeLayout(graph, { rankdir: 'LR' })
    expect(out.edges).toHaveLength(1)
    expect(out.edges[0].points.length).toBeGreaterThanOrEqual(2)
  })

  it('places target right of source under LR layout', () => {
    const out = computeLayout(graph, { rankdir: 'LR' })
    const a = out.nodes.find((n) => n.id === 'a')!
    const b = out.nodes.find((n) => n.id === 'b')!
    expect(b.x).toBeGreaterThan(a.x)
  })

  it('returns bounds wide enough for all nodes', () => {
    const out = computeLayout(graph, { rankdir: 'LR' })
    const maxX = Math.max(...out.nodes.map((n) => n.x + n.width / 2))
    expect(out.bounds.width).toBeGreaterThanOrEqual(maxX)
  })
})
