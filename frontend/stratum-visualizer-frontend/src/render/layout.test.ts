import { describe, expect, it } from 'vitest'
import { EdgeKind, GraphSnapshot } from '../types'
import { adaptSnapshot } from './adapt'
import { computeLayout } from './layout'

const snap: GraphSnapshot = {
  version: 1,
  modules: [
    { id: 1, path: 'src/a.ts', container: 10, layer: 0, stage: 1 },
    { id: 2, path: 'src/b.ts', container: 10, layer: 0, stage: 1 },
    { id: 3, path: 'src/c.ts', container: 11, layer: 1, stage: 1 },
  ],
  containers: [
    { id: 10, name: 'features', layer: 0, parent: null },
    { id: 11, name: 'shared', layer: 1, parent: null },
  ],
  layers: [
    { id: 0, name: 'features', depends_on: [1] },
    { id: 1, name: 'shared', depends_on: [] },
  ],
  edges: [{ from: 1, to: 3, kind: EdgeKind.Static }],
}

describe('computeLayout', () => {
  it('produces non-zero scene bounds', () => {
    const data = adaptSnapshot(snap)
    const scene = computeLayout(data)
    expect(scene.width).toBeGreaterThan(0)
    expect(scene.height).toBeGreaterThan(0)
  })

  it('emits a positioned module for every design module', () => {
    const data = adaptSnapshot(snap)
    const scene = computeLayout(data)
    expect(Object.keys(scene.modulePos).sort()).toEqual(['m-1', 'm-2', 'm-3'].sort())
    for (const pos of Object.values(scene.modulePos)) {
      expect(pos.w).toBeGreaterThan(0)
      expect(pos.h).toBeGreaterThan(0)
    }
  })

  it('lanes wrap into separate vertical bands', () => {
    const data = adaptSnapshot(snap)
    const scene = computeLayout(data)
    expect(scene.lanes).toHaveLength(2)
    expect(scene.lanes[1].y).toBeGreaterThan(scene.lanes[0].y)
  })

  it('resolves edge endpoints onto module centres', () => {
    const data = adaptSnapshot(snap)
    const scene = computeLayout(data)
    expect(scene.edges).toHaveLength(1)
    const e = scene.edges[0]
    const src = scene.modulePos['m-1']
    const dst = scene.modulePos['m-3']
    expect(e.x1).toBeCloseTo(src.cx)
    expect(e.x2).toBeCloseTo(dst.cx)
  })

  it('respects density tokens (compact < cozy < roomy)', () => {
    const data = adaptSnapshot(snap)
    const compact = computeLayout(data, { density: 'compact' })
    const cozy = computeLayout(data, { density: 'cozy' })
    const roomy = computeLayout(data, { density: 'roomy' })
    expect(compact.height).toBeLessThan(cozy.height)
    expect(cozy.height).toBeLessThan(roomy.height)
  })
})
