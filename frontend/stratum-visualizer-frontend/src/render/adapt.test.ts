import { describe, expect, it } from 'vitest'
import { EdgeKind, GraphSnapshot } from '../types'
import { adaptSnapshot } from './adapt'

const snap: GraphSnapshot = {
  version: 1,
  modules: [
    { id: 1, path: 'src/a.ts', container: 10, layer: 0, stage: 0 },
    { id: 2, path: 'src/b.ts', container: 10, layer: 1, stage: 0 },
  ],
  containers: [{ id: 10, name: 'pkg', layer: 0, parent: null }],
  layers: [
    { id: 0, name: 'domain', depends_on: [] },
    { id: 1, name: 'app', depends_on: [0] },
  ],
  edges: [{ from: 1, to: 2, kind: EdgeKind.Static }],
}

describe('adaptSnapshot', () => {
  it('emits one node per container and module', () => {
    const g = adaptSnapshot(snap)
    expect(g.nodes).toHaveLength(3)
  })

  it('maps edge endpoints to module ids', () => {
    const g = adaptSnapshot(snap)
    expect(g.edges).toEqual([{ from: 'm:1', to: 'm:2', kind: 'static' }])
  })

  it('assigns container as parent of contained modules', () => {
    const g = adaptSnapshot(snap)
    const mod = g.nodes.find((n) => n.id === 'm:1')!
    expect(mod.parent).toBe('c:10')
  })

  it('returns layer names in layer-id order', () => {
    const g = adaptSnapshot(snap)
    expect(g.layerNames).toEqual(['domain', 'app'])
  })
})
