import { describe, expect, it } from 'vitest'
import { snapshotV3 } from '../test/snapshot'
import { EdgeKind, Severity, SnapshotContainer, Violation } from '../types'
import { buildModel } from './model'

const container = (id: number, name: string, parent: number | null, entry: number | null = null): SnapshotContainer => ({
  id,
  name,
  path: parent === null ? `src/${name}` : `src/features/${name}`,
  kind: parent === null ? 'layer' : 'slice',
  layer: 0,
  parent,
  stage: 2,
  entry,
})

const SNAPSHOT = snapshotV3({
  layers: [{ id: 0, name: 'features', depends_on: [] }],
  containers: [container(10, 'features', null), container(11, 'cart', 10, 1), container(12, 'search', 10, 3)],
  modules: [
    { id: 1, path: 'src/features/cart/index.ts', container: 11, layer: 0, purity: 1 },
    { id: 2, path: 'src/features/cart/api.ts', container: 11, layer: 0, purity: 1 },
    { id: 3, path: 'src/features/search/index.ts', container: 12, layer: 0, purity: 1 },
  ],
  edges: [
    { from: 1, to: 2, kind: EdgeKind.Static },
    { from: 3, to: 2, kind: EdgeKind.Static },
  ],
})

const finding = (rule: string, modules: number[], edge: [number, number] | null): Violation => ({
  rule,
  severity: Severity.Error,
  message: rule,
  file: 'src/features/search/index.ts',
  location: { line: 1, column: 1 },
  modules,
  edge: edge && { from: edge[0], to: edge[1], kind: EdgeKind.Static },
  fix: null,
})

const FINDINGS = [finding('cross-slice', [3], [3, 2]), finding('purity', [2], null)]

describe('buildModel with a scope', () => {
  it('drops the modules, edges, emptied containers and findings the scope leaves out', () => {
    const keep = new Set([1, 2])
    const m = buildModel(SNAPSHOT, FINDINGS, { keeps: (id) => keep.has(id), isShared: (id) => id === 2 })

    expect([...m.modules.keys()]).toEqual([1, 2])
    expect(m.edges.map((e) => [e.from, e.to])).toEqual([[1, 2]])
    expect(m.inn(2)).toEqual([1])
    expect([...m.containers.keys()]).toEqual([10, 11])
    expect(m.units.map((u) => [u.id, u.modules])).toEqual([
      [10, []],
      [11, [1, 2]],
    ])
    expect(m.violations.map((v) => v.rule)).toEqual(['purity'])
    expect(m.isSharedByChosenFeatures(2)).toBe(true)
    expect(m.hides('src/features/search/index.ts')).toBe(true)
    expect(m.hides('src/features/search')).toBe(true)
    expect(m.hides('src/features/cart/api.ts')).toBe(false)
  })

  it('clears a container entry the scope hides', () => {
    const m = buildModel(SNAPSHOT, [], { keeps: (id) => id !== 1, isShared: () => false })
    expect(m.containers.get(11)?.entry).toBeNull()
    expect(m.containers.get(12)?.entry).toBe(3)
  })
})
