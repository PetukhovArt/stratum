import { describe, expect, it } from 'vitest'
import { buildModel } from '../../render/model'
import { snapshotV3 } from '../../test/snapshot'
import { EdgeKind, Severity, Violation } from '../../types'
import { layoutMatrix } from './layout'

const slice = (id: number, name: string) => ({
  id,
  name,
  path: `src/${name}`,
  kind: 'slice' as const,
  layer: 0,
  parent: null,
  stage: 2,
  entry: null,
})

const moduleIn = (id: number, container: number) => ({
  id,
  path: `src/m${id}.ts`,
  container,
  layer: 0,
  purity: 1,
})

const modelOf = (imports: [number, number][], violations: Violation[] = []) =>
  buildModel(
    snapshotV3({
      layers: [{ id: 0, name: 'app', depends_on: [] }],
      containers: [slice(10, 'a'), slice(11, 'b'), slice(12, 'c')],
      modules: [moduleIn(1, 10), moduleIn(2, 11), moduleIn(3, 12)],
      edges: imports.map(([from, to]) => ({ from, to, kind: EdgeKind.Static })),
    }),
    violations,
  )

const finding = (rule: string, from: number, to: number): Violation => ({
  rule,
  severity: Severity.Error,
  message: rule,
  file: `src/m${from}.ts`,
  location: { line: 1, column: 1 },
  modules: [from, to],
  edge: { from, to, kind: EdgeKind.Static },
  suggestion: null,
})

describe('layoutMatrix', () => {
  it('puts imported units before their importers', () => {
    const matrix = layoutMatrix(modelOf([[1, 2], [2, 3]]), 'unit')
    expect(matrix.ids).toEqual([12, 11, 10])
    expect(matrix.cells.some((c) => c.isAbove)).toBe(false)
  })

  it('draws an above-diagonal cell only where a cycle forces it', () => {
    const matrix = layoutMatrix(modelOf([[1, 2], [2, 1], [2, 3]]), 'unit')
    expect(matrix.cells.filter((c) => c.isAbove)).toHaveLength(1)
    expect(matrix.blocks).toEqual([{ start: 1, end: 2, size: 2 }])
  })

  it('counts only cross-layer and sibling findings as rule breaks', () => {
    const violations = [finding('stratum/public-api', 1, 2), finding('stratum/no-cross-layer-import', 2, 3)]
    const matrix = layoutMatrix(modelOf([[1, 2], [2, 3]], violations), 'unit')
    expect(matrix.drift.breaks).toBe(1)
    expect(matrix.cells.find((c) => c.from === 10)?.mark).toBeNull()
    expect(matrix.cells.find((c) => c.from === 11)?.mark).toBe('error')
  })
})
