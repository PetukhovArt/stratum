import { describe, expect, it } from 'vitest'
import { SCENE_MARK_BUDGET } from '../../render/budget'
import { buildModel } from '../../render/model'
import { snapshotV3 } from '../../test/snapshot'
import { EdgeKind, GraphSnapshot, Severity, SnapshotContainer, Territory, Violation } from '../../types'
import { layoutBands } from './bands'
import { COLUMN, ColumnsLayout, layoutColumns } from './columns'

const LAYERS = ['app', 'features', 'shared'].map((name, id) => ({ id, name, depends_on: [] }))

const layerContainer = (id: number): SnapshotContainer => ({
  id,
  name: LAYERS[id].name,
  path: `src/${LAYERS[id].name}`,
  kind: 'layer',
  layer: id,
  parent: null,
  stage: 2,
  entry: null,
})

const slice = (id: number, layer: number, name: string): SnapshotContainer => ({
  id,
  name,
  path: `src/${LAYERS[layer].name}/${name}`,
  kind: 'slice',
  layer,
  parent: layer,
  stage: 2,
  entry: null,
})

const finding = (from: number, to: number): Violation => ({
  rule: 'stratum/public-api',
  severity: Severity.Error,
  message: 'deep import',
  file: '',
  location: { line: 1, column: 1 },
  modules: [from, to],
  edge: { from, to, kind: EdgeKind.Static },
  fix: null,
})

const SHARED_API = 0

const sharedApiImportedBySlices = (slices: number, perSlice: number): GraphSnapshot =>
  sharedApiImportedBy(Array.from({ length: slices }, () => perSlice))

const sharedApiImportedBy = (sliceSizes: readonly number[]): GraphSnapshot => {
  const containers = [0, 1, 2].map(layerContainer)
  const modules = [{ id: SHARED_API, path: 'src/shared/api.ts', container: 2, layer: 2, purity: 2 }]
  const edges = []
  for (const [s, size] of sliceSizes.entries()) {
    const c = slice(10 + s, 1, `s${s}`)
    containers.push(c)
    for (let k = 0; k < size; k++) {
      const id = modules.length
      modules.push({ id, path: `${c.path}/m${k}.ts`, container: c.id, layer: 1, purity: 2 })
      edges.push({ from: id, to: SHARED_API, kind: EdgeKind.Static })
    }
  }
  return snapshotV3({ modules, containers, layers: LAYERS, edges })
}

const withAppModule = (base: GraphSnapshot, importsFrom: number): GraphSnapshot => {
  const id = base.modules.length
  return snapshotV3({
    ...base,
    modules: [...base.modules, { id, path: 'src/app/main.ts', container: 0, layer: 0, purity: 2 }],
    edges: [...base.edges, { from: id, to: importsFrom, kind: EdgeKind.Static }],
  })
}

describe('layoutColumns', () => {
  it('wraps a column taller than 880 px into sub-columns that each fit', () => {
    const layout = layoutColumns(buildModel(sharedApiImportedBySlices(60, 1), []), SHARED_API, 1, new Map())
    const importers = layout.columns.find((c) => c.side === -1)!

    expect(importers.subColumns).toBeGreaterThan(1)
    expect(importers.height).toBeLessThanOrEqual(COLUMN.maxHeight)
    expect(new Set(importers.groups.map((g) => g.x)).size).toBe(importers.subColumns)
  })

  it('closes a hop-1 group of more than 8 modules and bundles it into one edge', () => {
    const layout = layoutColumns(buildModel(sharedApiImportedBySlices(2, 9), []), SHARED_API, 1, new Map())
    const groups = layout.columns.find((c) => c.side === -1)!.groups

    expect(groups.map((g) => g.isOpen)).toEqual([false, false])
    expect(layout.bundles.map((b) => b.pairs.length)).toEqual([9, 9])
  })

  it('starts the 2nd hop collapsed even for a one-module group', () => {
    const snapshot = withAppModule(sharedApiImportedBySlices(1, 1), 1)
    const layout = layoutColumns(buildModel(snapshot, []), SHARED_API, 2, new Map())

    expect(layout.columns.find((c) => c.side === -1)!.groups[0].isOpen).toBe(true)
    expect(layout.columns.find((c) => c.side === -2)!.groups[0].isOpen).toBe(false)
  })

  it('bundles the 2nd hop group to group until the user expands the hop-1 group', () => {
    const withOne = withAppModule(sharedApiImportedBySlices(1, 2), 1)
    const app = withOne.modules.length - 1
    const snapshot = { ...withOne, edges: [...withOne.edges, { from: app, to: 2, kind: EdgeKind.Static }] }
    const m = buildModel(snapshot, [])
    const hopTwoPairs = (expanded: Map<string, boolean>) =>
      layoutColumns(m, SHARED_API, 2, expanded)
        .bundles.filter((b) => b.pairs.every(([from]) => from === app))
        .map((b) => b.pairs.length)
    const hopOneGroup = layoutColumns(m, SHARED_API, 2, new Map()).columns.find((c) => c.side === -1)!.groups[0]

    expect(hopOneGroup.isOpen).toBe(true)
    expect(hopTwoPairs(new Map())).toEqual([2])
    expect(hopTwoPairs(new Map([[hopOneGroup.key, true]]))).toEqual([1, 1])
  })

  it('marks the far end of a drawn edge with a finding, not a module whose finding is on an undrawn edge', () => {
    const snapshot = withAppModule(sharedApiImportedBySlices(1, 2), 2)
    const findings = [finding(1, SHARED_API), finding(3, 2)]
    const layout = layoutColumns(buildModel(snapshot, findings), SHARED_API, 1, new Map())
    const rows = layout.columns.find((c) => c.side === -1)!.groups[0].rows

    expect(rows.map((r) => [r.id, r.edgeFindings.length])).toEqual([
      [1, 1],
      [2, 0],
    ])
  })
})

describe('layoutColumns mark budget', () => {
  const GROUPS = 300
  const hub = buildModel(sharedApiImportedBy(Array.from({ length: GROUPS }, (_, i) => (i < 200 ? 7 : 6))), [])

  const FINDING_FREE_CARD_MARKS = 8
  const findingFreeMarks = (layout: ColumnsLayout): number =>
    FINDING_FREE_CARD_MARKS +
    layout.bundles.reduce((n, b) => n + 2 + Number(b.pairs.length > 1), 0) +
    layout.columns.reduce(
      (n, c) =>
        n +
        1 +
        c.heads.length * (Math.abs(c.side) === 1 ? 3 : 2) +
        c.groups.reduce((k, g) => k + 4 + g.rows.length * 2, 0),
      0,
    )

  it('starts a hub of 2,000 importers in 300 groups within the mark budget, every group still counted', () => {
    const layout = layoutColumns(hub, SHARED_API, 2, new Map())
    const importers = layout.columns.find((c) => c.side === -1)!

    expect(importers.count).toBe(2000)
    expect(findingFreeMarks(layout)).toBeLessThanOrEqual(SCENE_MARK_BUDGET)
    expect(importers.groups.length + layout.fold.hidden).toBe(GROUPS)
  })

  it('opens a group the user expanded even when the budget collapsed it', () => {
    const collapsed = layoutColumns(hub, SHARED_API, 2, new Map()).columns.find((c) => c.side === -1)!.groups[0]
    const expanded = layoutColumns(hub, SHARED_API, 2, new Map([[collapsed.key, true]]))
    const group = expanded.columns.find((c) => c.side === -1)!.groups.find((g) => g.key === collapsed.key)!

    expect(collapsed.isOpen).toBe(false)
    expect(group.isOpen).toBe(true)
    expect(group.rows.length).toBe(collapsed.ids.length)
  })
})

describe('layoutBands', () => {
  it('shades each half from the territory and keeps allowed layers without imports', () => {
    const territory: Territory = {
      may_import_layers: [0, 2],
      imported_by_layers: [1],
      max_import_purity: 4,
      visible_zones: [],
      may_not: [],
    }
    const snapshot = { ...sharedApiImportedBySlices(1, 1), territories: [territory] }
    const layout = layoutBands(buildModel(snapshot, []), SHARED_API)

    expect(
      layout.bands.map((b) => [b.layer.name, b.isImportAllowed, b.isImportedByAllowed, b.imports.length]),
    ).toEqual([
      ['app', true, false, 0],
      ['features', false, true, 0],
      ['shared', true, false, 0],
    ])
    expect(layout.bands[1].importers).toHaveLength(1)
  })
})
