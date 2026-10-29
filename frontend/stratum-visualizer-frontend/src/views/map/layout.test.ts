import { describe, expect, it } from 'vitest'
import { buildModel, Model } from '../../render/model'
import { snapshotV3 } from '../../test/snapshot'
import { SnapshotContainer, SnapshotLayer } from '../../types'
import { BlocksLayout, layerRanks, layoutBlocks, moveUnits, Rect } from './layout'

const LAYERS: SnapshotLayer[] = [
  { id: 0, name: 'shared', depends_on: [] },
  { id: 1, name: 'domain', depends_on: [0] },
  { id: 2, name: 'features', depends_on: [1, 0] },
  { id: 3, name: 'app', depends_on: [2, 1, 0] },
  { id: 4, name: 'editor-core', depends_on: [0] },
  { id: 5, name: 'editor-ui', depends_on: [4, 0] },
]
const UNITS_PER_LAYER = [9, 4, 14, 1, 1, 2]

const modelOf = (): Model => {
  const containers: SnapshotContainer[] = []
  const modules: { id: number; path: string; container: number; layer: number; purity: number }[] = []
  for (const layer of LAYERS) {
    const root = 100 + layer.id * 100
    containers.push({ id: root, name: layer.name, path: `src/${layer.name}`, kind: 'layer', layer: layer.id, parent: null, stage: 2, entry: null })
    for (let i = 0; i < UNITS_PER_LAYER[layer.id]; i++) {
      const id = root + 1 + i
      const path = `src/${layer.name}/unit-${'x'.repeat(i % 5)}${i}`
      containers.push({ id, name: path.split('/').pop() ?? '', path, kind: 'slice', layer: layer.id, parent: root, stage: 2, entry: null })
      for (let k = 0; k < 1 + ((i * 7) % 30); k++) {
        modules.push({ id: modules.length, path: `${path}/m${k}.ts`, container: id, layer: layer.id, purity: 1 })
      }
    }
  }
  return buildModel(snapshotV3({ layers: LAYERS, containers, modules, edges: [] }), [])
}

const overlaps = (a: Rect, b: Rect): boolean =>
  a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h

const contains = (outer: Rect, inner: Rect): boolean =>
  inner.x >= outer.x && inner.y >= outer.y && inner.x + inner.w <= outer.x + outer.w && inner.y + inner.h <= outer.y + outer.h

const pairs = <T>(xs: readonly T[]): [T, T][] => xs.flatMap((a, i) => xs.slice(i + 1).map((b): [T, T] => [a, b]))

const expectReadable = (layout: BlocksLayout): void => {
  const boxes = [...layout.boxes.values()]
  expect(pairs(boxes).filter(([a, b]) => overlaps(a, b))).toEqual([])
  expect(pairs([...layout.blocks]).filter(([a, b]) => overlaps(a, b))).toEqual([])
  for (const block of layout.blocks) {
    for (const id of block.units) expect(contains(block, layout.boxes.get(id)!)).toBe(true)
  }
  for (const block of layout.blocks) expect(contains(layout.bounds, block)).toBe(true)
}

describe('layerRanks', () => {
  it('counts the longest depends_on chain below each layer and stops at a cycle', () => {
    const ranks = layerRanks([...LAYERS, { id: 6, name: 'a', depends_on: [7] }, { id: 7, name: 'b', depends_on: [6, 3] }])
    expect([0, 1, 2, 3, 4, 5].map((id) => ranks.get(id))).toEqual([0, 1, 2, 3, 1, 2])
    expect(ranks.get(7)).toBeGreaterThan(3)
  })
})

describe('layoutBlocks', () => {
  it('packs every unit into its layer block with no two boxes or blocks overlapping', () => {
    const m = modelOf()
    expectReadable(layoutBlocks(m, new Set()))
    expectReadable(layoutBlocks(m, new Set([m.units.find((u) => u.modules.length > 20)!.id])))
  })

  it('reads from the layers that depend on the most down to those that depend on nothing', () => {
    const order = layoutBlocks(modelOf(), new Set()).blocks.map((b) => LAYERS[b.layer].name)
    expect(order[0]).toBe('app')
    expect(order[order.length - 1]).toBe('shared')
    expect(order.indexOf('features')).toBeLessThan(order.indexOf('domain'))
  })

  it('gives the same layout for the same model', () => {
    expect(layoutBlocks(modelOf(), new Set())).toEqual(layoutBlocks(modelOf(), new Set()))
  })

  it('keeps a reserved slot clear of every block', () => {
    const layout = layoutBlocks(modelOf(), new Set(), { w: 200, h: 140 })
    const slot = layout.reserved!
    expect(slot).toMatchObject({ w: 200, h: 140 })
    expect(layout.blocks.filter((b) => overlaps(b, slot))).toEqual([])
    expect(contains(layout.bounds, slot)).toBe(true)
  })
})

describe('moveUnits', () => {
  it('moves a unit and stretches only its own block frame and the bounds to follow it', () => {
    const base = layoutBlocks(modelOf(), new Set())
    const block = base.blocks.find((b) => b.units.length > 1)!
    const id = block.units[0]
    const far = { x: base.bounds.x + base.bounds.w + 500, y: base.bounds.y + base.bounds.h + 300 }
    const moved = moveUnits(base, new Map([[id, far]]))
    expect(moved.boxes.get(id)).toMatchObject(far)
    const frame = moved.blocks.find((b) => b.layer === block.layer)!
    for (const unit of frame.units) expect(contains(frame, moved.boxes.get(unit)!)).toBe(true)
    expect(moved.blocks.filter((b) => b.layer !== block.layer)).toEqual(base.blocks.filter((b) => b.layer !== block.layer))
    expect(contains(moved.bounds, moved.boxes.get(id)!)).toBe(true)
  })
})
