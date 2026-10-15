import { describe, expect, it } from 'vitest'
import { parseSnapshot, SnapshotVersionMismatchError } from './snapshot'

describe('parseSnapshot', () => {
  it('rejects a v1 snapshot whose modules carry `stage` instead of `purity`', () => {
    const v1 = {
      version: 1,
      modules: [{ id: 1, path: 'a.ts', container: 0, layer: 0, stage: 2 }],
      containers: [{ id: 0, name: 'a', layer: 0, parent: null }],
      layers: [],
      edges: [],
    }
    expect(() => parseSnapshot(v1)).toThrow(SnapshotVersionMismatchError)
  })
})
