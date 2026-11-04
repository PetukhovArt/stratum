import { describe, expect, it } from 'vitest'
import { buildModel, ModuleScope } from '../../render/model'
import { snapshotV3 } from '../../test/snapshot'
import { EdgeKind, SnapshotContainer } from '../../types'
import { DriftReport } from './drift'
import { layoutFolder, layoutOverview, routeFolds } from './layout'
import { linesOfModule, toReview } from './review'

const container = (id: number, name: string, path: string, kind: SnapshotContainer['kind'], parent: number | null) => ({
  id,
  name,
  path,
  kind,
  layer: id === 1 ? 1 : 0,
  parent,
  stage: 2,
  entry: null,
})

const model = (scope?: ModuleScope) =>
  buildModel(
    snapshotV3({
      layers: [
        { id: 0, name: 'features', depends_on: [1] },
        { id: 1, name: 'shared', depends_on: [] },
      ],
      containers: [
        container(0, 'features', 'src/features', 'layer', null),
        container(1, 'shared', 'src/shared', 'layer', null),
        container(2, 'cart', 'src/features/cart', 'slice', 0),
      ],
      modules: [
        { id: 0, path: 'src/features/cart/index.ts', container: 2, layer: 0, purity: 2 },
        { id: 1, path: 'src/features/cart/model.ts', container: 2, layer: 0, purity: 2 },
        { id: 2, path: 'src/shared/api.ts', container: 1, layer: 1, purity: 2 },
      ],
      edges: [
        { from: 0, to: 1, kind: EdgeKind.Static },
        { from: 0, to: 2, kind: EdgeKind.Static },
      ],
    }),
    [],
    scope,
  )

const drift: DriftReport = {
  base: { ref: 'HEAD', sha: 'abc1234' },
  markdown: '<!-- stratum:drift -->\n',
  breaking: [0],
  edges: [
    { from: 'src/features/cart/index.ts', to: 'src/shared/api.ts', kind: EdgeKind.Static, change: 'added', breaks: [['stratum/public-api', 'error']] },
    { from: 'src/features/cart/index.ts', to: 'src/features/cart/old.ts', kind: EdgeKind.Static, change: 'removed', breaks: [] },
    { from: 'src/features/cart/model.ts', to: 'src/features/cart/old.ts', kind: EdgeKind.Static, change: 'removed', breaks: [] },
  ],
  cycles: [],
  stages: [],
}

describe('toReview', () => {
  it('draws a removed import of a deleted file to a ghost placed in the container of its path', () => {
    const m = model()
    const review = toReview(m, drift)

    const ghost = [...review.nodes.values()].find((n) => n.isGhost)
    expect(ghost).toMatchObject({ path: 'src/features/cart/old.ts', layer: 0, slice: 2 })
    expect(review.changes.find((e) => e.to === ghost?.id)?.tag).toBe('removed')
    expect(layoutFolder(m, review, ghost!.group.key).nodes.has(ghost!.id)).toBe(true)
  })

  it('draws the unchanged imports of a selected module as context and lists each importer once', () => {
    const m = model()
    const review = toReview(m, drift)

    expect(linesOfModule(m, review, 0).map((e) => [e.from, e.to, e.tag])).toContainEqual([0, 1, 'same'])
    expect(review.files).toEqual([
      { id: 0, path: 'src/features/cart/index.ts', isGhost: false, added: 1, removed: 1, worst: 'error' },
      { id: 1, path: 'src/features/cart/model.ts', isGhost: false, added: 0, removed: 1, worst: null },
    ])
  })

  it('leaves out changes touching a module the feature filter hides, not drawing it as a ghost', () => {
    const toShared = drift.edges[0]
    const intoCart = { ...toShared, from: 'src/features/cart/model.ts', to: 'src/features/cart/index.ts' }
    const m = model({ chosen: [], keeps: (id) => id !== 2, isShared: () => false })
    const review = toReview(m, { ...drift, breaking: [0, 1], edges: [toShared, intoCart] })

    expect(review.counts).toMatchObject({ added: 1, breaking: 1 })
    expect(review.items).toEqual([
      expect.objectContaining({ kind: 'import', from: 'src/features/cart/model.ts', to: 'src/features/cart/index.ts' }),
    ])
    expect([...review.nodes.values()].some((n) => n.isGhost)).toBe(false)
  })

  it('folds the changes between two folders into one arrow apart from rule breaks, and counts each folder’s outgoing changes and breaks', () => {
    const plain = { ...drift.edges[0], from: 'src/features/cart/model.ts', breaks: [] }
    const m = model()
    const review = toReview(m, { ...drift, edges: [...drift.edges, plain] })

    expect(review.alwaysDrawn.map((e) => [e.from, e.to])).toEqual([[0, 2]])
    expect(review.folds).toEqual([expect.objectContaining({ from: 'c2', to: 'c1', added: 1, removed: 0 })])
    expect(review.folderOf.get('c2')).toMatchObject({ added: 2, removed: 2, breaks: 1, worst: 'error', ghosts: 1 })
    const overview = layoutOverview(m, review)
    expect([routeFolds(overview, review.folds).length, overview.breaks.length, overview.boxes.length]).toEqual([1, 1, 2])
  })
})
