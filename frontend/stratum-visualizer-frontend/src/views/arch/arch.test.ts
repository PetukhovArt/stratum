import { describe, expect, it } from 'vitest'
import { buildModel, Model } from '../../render/model'
import { snapshotV3 } from '../../test/snapshot'
import { EdgeKind, SnapshotContainer } from '../../types'
import { layoutComponents, marksWithinBudget, segmentsOf } from './components'
import { ArchPath, crumbsOf, deepestShownPrefix, downOf, parseArchPath, upOf } from './path'

const container = (id: number, name: string, path: string, kind: SnapshotContainer['kind'], parent: number | null) => ({
  id,
  name,
  path,
  kind,
  layer: 0,
  parent,
  stage: 2,
  entry: null,
})

const modules = [
  'src/features/cart/index.ts',
  'src/features/cart/ui/Cart.tsx',
  'src/features/cart/ui/forms/Address.tsx',
  'src/features/cart/model/store.ts',
  'src/features/cart/model/nested/deep.ts',
  'src/features/search/index.ts',
]

const SNAPSHOT = snapshotV3({
  layers: [{ id: 0, name: 'features', depends_on: [] }],
  containers: [
    container(10, 'features', 'src/features', 'layer', null),
    container(11, 'cart', 'src/features/cart', 'slice', 10),
    container(12, 'nested', 'src/features/cart/model/nested', 'nested', 11),
    container(13, 'search', 'src/features/search', 'slice', 10),
  ],
  modules: modules.map((path, id) => ({
    id,
    path,
    container: path.includes('nested') ? 12 : path.includes('search') ? 13 : 11,
    layer: 0,
    purity: 1,
  })),
  edges: [
    { from: 1, to: 3, kind: EdgeKind.Static },
    { from: 2, to: 3, kind: EdgeKind.Static },
    { from: 0, to: 1, kind: EdgeKind.Static },
    { from: 5, to: 0, kind: EdgeKind.Static },
  ],
})

const model: Model = buildModel(SNAPSHOT, [])
const cart = model.units.find((u) => u.id === 11)!

describe('Components level', () => {
  it('groups a container by its first sub-folder, keeps nested containers in theirs and files at the root apart', () => {
    expect(segmentsOf(model, cart).map((s) => [s.name, s.modules.map((id) => modules[id])])).toEqual([
      ['model', ['src/features/cart/model/nested/deep.ts', 'src/features/cart/model/store.ts']],
      ['ui', ['src/features/cart/ui/Cart.tsx', 'src/features/cart/ui/forms/Address.tsx']],
      ['', ['src/features/cart/index.ts']],
    ])
  })

  it('draws only the imports inside the container and sums those that cross segments', () => {
    const layout = layoutComponents(model, cart)
    expect(layout.imports.map((a) => [a.from, a.to]).sort((x, y) => x[0] - y[0])).toEqual([
      [0, 1],
      [1, 3],
      [2, 3],
    ])
    expect(layout.segmentLinks.map((l) => [l.from, l.to, l.count])).toEqual([
      ['ui', 'model', 2],
      ['', 'ui', 1],
    ])
    expect(layout.nodes.get(2)?.label).toBe('forms/Address.tsx')
  })

  it('keeps every import within the label budget and past it only the imports of the selected module', () => {
    const chain = (n: number) => {
      const m = buildModel(
        snapshotV3({
          layers: [{ id: 0, name: 'shared', depends_on: [] }],
          containers: [container(0, 'shared', 'src/shared', 'layer', null)],
          modules: Array.from({ length: n }, (_, id) => ({ id, path: `src/shared/lib/m${id}.ts`, container: 0, layer: 0, purity: 1 })),
          edges: Array.from({ length: n - 1 }, (_, i) => ({ from: i, to: i + 1, kind: EdgeKind.Static })),
        }),
        [],
      )
      return layoutComponents(m, m.units[0])
    }
    expect(marksWithinBudget(chain(100), null)).toMatchObject({ hidden: 0, hasLabels: true })
    const past = marksWithinBudget(chain(600), 10)
    expect(past.imports.map((a) => [a.from, a.to]).sort((x, y) => x[0] - y[0])).toEqual([
      [9, 10],
      [10, 11],
    ])
    expect(past.hidden).toBe(599 - 2)
  })
})

describe('architecture path', () => {
  const code: ArchPath = { level: 'code', container: 11, module: 1 }

  const self = { id: 'ui', name: 'Web UI', contractsTotal: 0, unavailableReason: null }

  it('derives one breadcrumb per level, goes up one level at a time and down into the selection', () => {
    expect(crumbsOf(code, model, null).map((c) => [c.level, c.label])).toEqual([
      ['containers', 'test'],
      ['components', 'cart'],
      ['code', 'Cart.tsx'],
    ])
    expect(upOf(code, null)).toEqual({ level: 'components', container: 11 })
    expect(upOf({ level: 'components', container: 11 }, null)).toEqual({ level: 'containers' })
    expect(upOf({ level: 'containers' }, null)).toBeNull()
    expect(downOf({ level: 'containers' }, { kind: 'container', id: 13 }, null)).toEqual({ level: 'components', container: 13 })
    expect(downOf({ level: 'components', container: 11 }, { kind: 'module', id: 2 }, null)).toEqual({
      level: 'code',
      container: 11,
      module: 2,
    })
    expect(downOf({ level: 'containers' }, { kind: 'cell', level: 'unit', from: 11, to: 13 }, null)).toBeNull()
  })

  it('puts the System level on top when systems are declared and enters only the self system', () => {
    expect(crumbsOf(code, model, self).map((c) => [c.level, c.label])).toEqual([
      ['system', 'Web UI'],
      ['containers', 'test'],
      ['components', 'cart'],
      ['code', 'Cart.tsx'],
    ])
    expect(crumbsOf({ level: 'system' }, model, self).map((c) => c.level)).toEqual(['system'])
    expect(upOf({ level: 'containers' }, self)).toEqual({ level: 'system' })
    expect(upOf({ level: 'system' }, self)).toBeNull()
    expect(downOf({ level: 'system' }, { kind: 'system', id: 'ui' }, self)).toEqual({ level: 'containers' })
    expect(downOf({ level: 'system' }, { kind: 'system', id: 'api' }, self)).toBeNull()
    expect(downOf({ level: 'system' }, { kind: 'channel', to: 'api' }, self)).toBeNull()
  })

  it('restores every level from storage and shows System only when systems are declared', () => {
    const paths: ArchPath[] = [{ level: 'system' }, { level: 'containers' }, { level: 'components', container: 11 }, code]
    for (const path of paths) expect(parseArchPath(JSON.parse(JSON.stringify(path)))).toEqual(path)
    expect(parseArchPath({ level: 'components' })).toEqual({ level: 'system' })
    expect(deepestShownPrefix({ level: 'system' }, model, self)).toEqual({ level: 'system' })
    expect(deepestShownPrefix({ level: 'system' }, model, null)).toEqual({ level: 'containers' })
  })

  it('falls back to the deepest level the filtered model still holds', () => {
    const searchOnly = buildModel(SNAPSHOT, [], { keeps: (id) => id === 5, isShared: () => false })
    expect(deepestShownPrefix(code, searchOnly, self)).toEqual({ level: 'containers' })
    expect(deepestShownPrefix({ level: 'code', container: 11, module: 5 }, model, self)).toEqual({
      level: 'components',
      container: 11,
    })
    expect(deepestShownPrefix(code, model, self)).toBe(code)
  })
})
