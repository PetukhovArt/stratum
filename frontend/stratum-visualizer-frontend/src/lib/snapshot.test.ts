import { describe, expect, it } from 'vitest'
import { parseSnapshot, SnapshotVersionMismatchError } from './snapshot'

describe('parseSnapshot', () => {
  it('rejects a v2 snapshot, whose modules carry absolute paths and no territory', () => {
    const v2 = {
      version: 2,
      modules: [{ id: 1, path: 'C:\\repo\\src\\a.ts', container: 0, layer: 0, purity: 2 }],
      containers: [{ id: 0, name: 'a', path: 'C:\\repo\\src', kind: 'layer', layer: 0, parent: null, stage: 2, entry: null }],
      layers: [],
      edges: [],
    }
    expect(() => parseSnapshot(v2)).toThrow(SnapshotVersionMismatchError)
  })
})
