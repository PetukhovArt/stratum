import { describe, expect, it } from 'vitest'
import { buildModel, Model } from '../../render/model'
import { snapshotV3 } from '../../test/snapshot'
import { EdgeKind, SnapshotContainer } from '../../types'
import { layoutComponents, marksWithinBudget, segmentsOf } from './components'
import { ArchPath, crumbsOf, deepestShownPrefix, downOf, upOf } from './path'

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

  it('derives one breadcrumb per level, goes up one level at a time and down into the selection', () => {
    expect(crumbsOf(code, model).map((c) => [c.level, c.label])).toEqual([
      ['containers', 'test'],
      ['components', 'cart'],
      ['code', 'Cart.tsx'],
    ])
    expect(upOf(code)).toEqual({ level: 'components', container: 11 })
    expect(upOf({ level: 'components', container: 11 })).toEqual({ level: 'containers' })
    expect(upOf({ level: 'containers' })).toBeNull()
    expect(downOf({ level: 'containers' }, { kind: 'container', id: 13 })).toEqual({ level: 'components', container: 13 })
    expect(downOf({ level: 'components', container: 11 }, { kind: 'module', id: 2 })).toEqual({
      level: 'code',
      container: 11,
      module: 2,
    })
    expect(downOf({ level: 'containers' }, { kind: 'cell', level: 'unit', from: 11, to: 13 })).toBeNull()
  })

  it('falls back to the deepest level the filtered model still holds', () => {
    const searchOnly = buildModel(SNAPSHOT, [], { keeps: (id) => id === 5, isShared: () => false })
    expect(deepestShownPrefix(code, searchOnly)).toEqual({ level: 'containers' })
    expect(deepestShownPrefix({ level: 'code', container: 11, module: 5 }, model)).toEqual({
      level: 'components',
      container: 11,
    })
    expect(deepestShownPrefix(code, model)).toBe(code)
  })
})
