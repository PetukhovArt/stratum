import { describe, expect, it } from 'vitest'
import { RenderGraph } from './adapt'
import { sliceIfNeeded, SLICE_THRESHOLD } from './sliced'

const bigGraph = (n: number): RenderGraph => ({
  nodes: Array.from({ length: n }, (_, i) => ({
    id: `n${i}`,
    label: `n${i}`,
    layer: i % 3,
    kind: 'module',
    parent: null,
  })),
  edges: [],
  layerNames: ['l0', 'l1', 'l2'],
})

describe('sliceIfNeeded', () => {
  it('returns input unchanged below the threshold', () => {
    const out = sliceIfNeeded(bigGraph(SLICE_THRESHOLD - 1))
    expect(out.sliced).toBe(false)
  })

  it('collapses to per-layer aggregates above the threshold', () => {
    const out = sliceIfNeeded(bigGraph(SLICE_THRESHOLD + 50))
    expect(out.sliced).toBe(true)
    expect(out.graph.nodes.length).toBeLessThanOrEqual(3)
  })

  it('preserves layer names in collapsed view', () => {
    const out = sliceIfNeeded(bigGraph(SLICE_THRESHOLD + 50))
    const labels = out.graph.nodes.map((n) => n.label)
    for (const layer of ['l0', 'l1', 'l2']) {
      expect(labels.some((l) => l.includes(layer))).toBe(true)
    }
  })

  it('expanded layers render in full, others stay aggregated', () => {
    const out = sliceIfNeeded(bigGraph(SLICE_THRESHOLD + 50), new Set([0]))
    const aggregates = out.graph.nodes.filter((n) => n.kind === 'layer-aggregate')
    const modules = out.graph.nodes.filter((n) => n.kind === 'module')
    expect(aggregates).toHaveLength(2)
    expect(modules.every((m) => m.layer === 0)).toBe(true)
  })
})
