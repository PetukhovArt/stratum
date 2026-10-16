import { describe, expect, it } from 'vitest'
import { buildModel } from '../../render/model'
import { snapshotV3 } from '../../test/snapshot'
import { EdgeKind } from '../../types'
import { analyzeSet } from './analysis'

const modelOf = (ids: number[], imports: [number, number][]) =>
  buildModel(
    snapshotV3({
      layers: [{ id: 0, name: 'app', depends_on: [] }],
      containers: [{ id: 100, name: 'app', path: 'src', kind: 'layer', layer: 0, parent: null, stage: 2, entry: null }],
      modules: ids.map((id) => ({ id, path: `src/m${id}.ts`, container: 100, layer: 0, purity: 1 })),
      edges: imports.map(([from, to]) => ({ from, to, kind: EdgeKind.Static })),
    }),
    [],
  )

describe('analyzeSet', () => {
  it('scores a cut by the loop members that leave the largest cycle when only that import is removed', () => {
    const [a, b, c] = [1, 2, 3]
    const m = modelOf([a, b, c], [
      [a, b],
      [b, c],
      [c, a],
      [b, a],
    ])
    const scores = Object.fromEntries(analyzeSet(m, [a, b, c]).cuts.map((x) => [`${x.from}>${x.to}`, x.freed]))
    expect(scores).toEqual({ '1>2': 3, '2>3': 1, '3>1': 1, '2>1': 0 })
  })

  it('finds connecting paths through non-members only, never through another member', () => {
    const [a, member, c, y, p, q, r] = [1, 2, 3, 10, 11, 12, 13]
    const m = modelOf([a, member, c, y, p, q, r], [
      [a, y],
      [y, member],
      [member, c],
      [a, p],
      [p, q],
      [q, r],
      [r, c],
    ])
    const { paths, via } = analyzeSet(m, [a, member, c])
    expect(paths).toEqual([
      [a, y, member],
      [a, p, q, r, c],
    ])
    expect(new Set(via)).toEqual(new Set([y, p, q, r]))
  })
})
