import { describe, expect, it } from 'vitest'
import { EdgeKind, GraphSnapshot, Severity, Violation } from '../types'
import { adaptSnapshot } from './adapt'

const snap: GraphSnapshot = {
  version: 1,
  modules: [
    { id: 1, path: 'src/features/auth/index.ts', container: 10, layer: 0, stage: 2 },
    { id: 2, path: 'src/features/auth/_shared.ts', container: 10, layer: 0, stage: 1 },
    { id: 3, path: 'src/shared/lib/result.ts', container: 11, layer: 1, stage: 1 },
  ],
  containers: [
    { id: 10, name: 'features/auth', layer: 0, parent: null },
    { id: 11, name: 'shared/lib', layer: 1, parent: null },
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
    const d = adaptSnapshot(snap)
    expect(d.modules).toHaveLength(3)
    expect(d.modules.map((m) => m.id).sort()).toEqual(['m-1', 'm-2', 'm-3'].sort())
  })

  it('emits a design container for every top-level snapshot container', () => {
    const d = adaptSnapshot(snap)
    expect(d.containers.map((c) => c.id).sort()).toEqual(['c-10', 'c-11'].sort())
  })

  it('infers composer/shared/regular kind from basename', () => {
    const d = adaptSnapshot(snap)
    expect(d.modules.find((m) => m.id === 'm-1')?.kind).toBe('composer')
    expect(d.modules.find((m) => m.id === 'm-2')?.kind).toBe('shared')
    expect(d.modules.find((m) => m.id === 'm-3')?.kind).toBe('regular')
  })

  it('rewrites edges with module-key endpoints and edge-kind labels', () => {
    const d = adaptSnapshot(snap)
    expect(d.edges).toEqual([
      { source: 'm-1', target: 'm-3', kind: 'static', violation: null },
      { source: 'm-2', target: 'm-3', kind: 'di', violation: null },
    ])
  })

  it('lists layers in snapshot order with deterministic hues', () => {
    const d = adaptSnapshot(snap)
    expect(d.layers.map((l) => l.label)).toEqual(['features', 'shared'])
    expect(d.layers.every((l) => typeof l.hue === 'number')).toBe(true)
  })

  it('attaches violations to involved modules and propagates severity', () => {
    const violations: Violation[] = [
      {
        rule: 7,
        severity: Severity.Error,
        message: 'cross-layer',
        file: 'x',
        location: { line: 1, column: 1 },
        modules: [3],
        edge: { from: 1, to: 3, kind: EdgeKind.Static },
        suggestion: null,
      },
    ]
    const d = adaptSnapshot(snap, violations)
    const m3 = d.modules.find((m) => m.id === 'm-3')!
    expect(m3.severity).toBe('error')
    expect(m3.violations).toHaveLength(1)
    // edge endpoints are tagged too because the violation has an `edge`
    expect(d.modules.find((m) => m.id === 'm-1')!.severity).toBe('error')
    // edge picks up the violation tag
    expect(d.edges.find((e) => e.source === 'm-1' && e.target === 'm-3')!.violation).toMatch(
      /^v-\d+$/,
    )
  })

  it('maps nested containers to compound modules under their root', () => {
    const nested: GraphSnapshot = {
      ...snap,
      containers: [
        { id: 20, name: 'features', layer: 0, parent: null },
        { id: 21, name: 'auth', layer: 0, parent: 20 },
      ],
      modules: [{ id: 1, path: 'src/features/auth/x.ts', container: 21, layer: 0, stage: 1 }],
      edges: [],
    }
    const d = adaptSnapshot(nested)
    // top-level container 20 becomes the design container
    expect(d.containers.map((c) => c.id)).toEqual(['c-20'])
    // nested container becomes a compound module attached under c-20
    const compound = d.modules.find((m) => m.id === 'cn-21')!
    expect(compound.hasChildren).toBe(true)
    expect(compound.container).toBe('c-20')
    expect(d.topByContainer['c-20']).toContain('cn-21')
    expect(d.childIndex['cn-21']).toContain('m-1')
  })
})
