import { describe, expect, it } from 'vitest'
import { EdgeKind, Severity, Violation } from '../types'
import { designOf, SnapshotInput } from '../test/snapshot'

const snap: SnapshotInput = {
  modules: [
    { id: 1, path: 'src/features/auth/index.ts', container: 10, layer: 0, purity: 2 },
    { id: 2, path: 'src/features/auth/_shared.ts', container: 10, layer: 0, purity: 1 },
    { id: 3, path: 'src/shared/lib/result.ts', container: 11, layer: 1, purity: 1 },
  ],
  containers: [
    { id: 10, name: 'features/auth', path: 'src/features/auth', kind: 'layer', layer: 0, parent: null, stage: 2, entry: null },
    { id: 11, name: 'shared/lib', path: 'src/shared/lib', kind: 'layer', layer: 1, parent: null, stage: 2, entry: null },
  ],
  layers: [
    { id: 0, name: 'features', depends_on: [1] },
    { id: 1, name: 'shared', depends_on: [] },
  ],
  edges: [
    { from: 1, to: 3, kind: EdgeKind.Static },
    { from: 2, to: 3, kind: EdgeKind.Di },
  ],
}

describe('adaptSnapshot', () => {
  it('maps every snapshot module to a design module', () => {
    const d = designOf(snap)
    expect(d.modules).toHaveLength(3)
    expect(d.modules.map((m) => m.id).sort()).toEqual(['m-1', 'm-2', 'm-3'].sort())
  })

  it('emits a design container for every top-level snapshot container', () => {
    const d = designOf(snap)
    expect(d.containers.map((c) => c.id).sort()).toEqual(['c-10', 'c-11'].sort())
  })

  it('infers composer/shared/regular kind from basename', () => {
    const d = designOf(snap)
    expect(d.modules.find((m) => m.id === 'm-1')?.kind).toBe('composer')
    expect(d.modules.find((m) => m.id === 'm-2')?.kind).toBe('shared')
    expect(d.modules.find((m) => m.id === 'm-3')?.kind).toBe('regular')
  })

  it('rewrites edges with module-key endpoints and edge-kind labels', () => {
    const d = designOf(snap)
    expect(d.edges).toEqual([
      { source: 'm-1', target: 'm-3', kind: 'static', violation: null, severity: null },
      { source: 'm-2', target: 'm-3', kind: 'di', violation: null, severity: null },
    ])
  })

  it('lists layers in snapshot order with stable, distinct hues', () => {
    const d = designOf(snap)
    expect(d.layers.map((l) => l.label)).toEqual(['features', 'shared'])
    const hues = d.layers.map((l) => l.hue)
    expect(designOf(snap).layers.map((l) => l.hue)).toEqual(hues)
    expect(new Set(hues).size).toBe(hues.length)
  })

  it('attaches violations to involved modules and propagates severity', () => {
    const violations: Violation[] = [
      {
        rule: 'stratum/no-cross-layer-import',
        severity: Severity.Error,
        message: 'cross-layer',
        file: 'x',
        location: { line: 1, column: 1 },
        modules: [3],
        edge: { from: 1, to: 3, kind: EdgeKind.Static },
        suggestion: null,
      },
    ]
    const d = designOf(snap, violations)
    const m3 = d.modules.find((m) => m.id === 'm-3')!
    expect(m3.severity).toBe('error')
    expect(m3.violations).toHaveLength(1)
    expect(d.modules.find((m) => m.id === 'm-1')!.severity).toBe('error')
    expect(d.edges.find((e) => e.source === 'm-1' && e.target === 'm-3')!.violation).toMatch(
      /^v-\d+$/,
    )
  })

  it('maps nested containers to compound modules under their root', () => {
    const nested: SnapshotInput = {
      ...snap,
      containers: [
        { id: 20, name: 'features', path: 'src/features', kind: 'layer', layer: 0, parent: null, stage: 2, entry: null },
        { id: 21, name: 'auth', path: 'src/auth', kind: 'slice', layer: 0, parent: 20, stage: 2, entry: null },
      ],
      modules: [{ id: 1, path: 'src/features/auth/x.ts', container: 21, layer: 0, purity: 1 }],
      edges: [],
    }
    const d = designOf(nested)
    expect(d.containers.map((c) => c.id)).toEqual(['c-20'])
    const compound = d.modules.find((m) => m.id === 'cn-21')!
    expect(compound.hasChildren).toBe(true)
    expect(compound.container).toBe('c-20')
    expect(d.topByContainer['c-20']).toContain('cn-21')
    expect(d.childIndex['cn-21']).toContain('m-1')
  })

  it('shows purity on files and the backend stage on containers, never mixing the axes', () => {
    const composed: SnapshotInput = {
      ...snap,
      containers: [
        { id: 20, name: 'root', path: 'src/root', kind: 'layer', layer: 0, parent: null, stage: 2, entry: null },
        { id: 21, name: 'unit', path: 'src/root/unit', kind: 'sub_unit', layer: 0, parent: 20, stage: 4, entry: 1 },
      ],
      modules: [{ id: 1, path: 'src/root/unit/x.ts', container: 21, layer: 0, purity: 2 }],
      edges: [],
    }
    const d = designOf(composed)
    expect(d.moduleById.get('m-1')!.facet).toEqual({ axis: 'purity', value: 2 })
    expect(d.moduleById.get('cn-21')!.facet).toEqual({ axis: 'stage', value: 4, kind: 'sub-unit' })
    expect(d.containers[0].stage).toBe(2)
  })
})
