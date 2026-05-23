import { describe, expect, it } from 'vitest'
import { buildHitTest, HitTestIndex } from './hitTest'
import type { Scene } from '../layout'
import type { DesignModule } from '../design'

const mkMod = (id: string): DesignModule => ({
  id,
  label: id,
  path: id,
  kind: 'regular',
  loc: 10,
  container: 'c1',
  parentId: null,
  childIds: [],
  layer: 'app',
  stage: 0,
  severity: null,
  violations: [],
  descError: 0,
  descWarn: 0,
  hasChildren: false,
  descInfo: 0,
})

const mkScene = (): Scene => ({
  lanes: [],
  containers: [],
  edges: [],
  width: 1000,
  height: 1000,
  modulePos: {
    a: { x: 10, y: 10, w: 30, h: 30, cx: 25, cy: 25, mod: mkMod('a'), leaf: true, depth: 0, childIds: [] },
    b: { x: 100, y: 100, w: 40, h: 40, cx: 120, cy: 120, mod: mkMod('b'), leaf: true, depth: 0, childIds: [] },
    c: { x: 200, y: 200, w: 20, h: 20, cx: 210, cy: 210, mod: mkMod('c'), leaf: true, depth: 0, childIds: [] },
  },
})

describe('buildHitTest', () => {
  let idx: HitTestIndex
  it('returns an index', () => {
    idx = buildHitTest(mkScene())
    expect(idx).toBeDefined()
  })

  it('finds a module at its centre', () => {
    expect(idx.queryPoint(25, 25)).toBe('a')
    expect(idx.queryPoint(120, 120)).toBe('b')
  })

  it('returns null outside any module', () => {
    expect(idx.queryPoint(1000, 1000)).toBeNull()
    expect(idx.queryPoint(70, 70)).toBeNull()
  })

  it('finds modules at their edges (inclusive of x/y, exclusive of x+w/y+h)', () => {
    expect(idx.queryPoint(10, 10)).toBe('a')
    expect(idx.queryPoint(39.9, 39.9)).toBe('a')
    expect(idx.queryPoint(40, 40)).toBeNull()
  })

  it('returns the innermost (smallest) overlapping module when multiple match', () => {
    const overlap: Scene = {
      ...mkScene(),
      modulePos: {
        big: { x: 0, y: 0, w: 100, h: 100, cx: 50, cy: 50, mod: mkMod('big'), leaf: false, depth: 0, childIds: ['small'] },
        small: { x: 40, y: 40, w: 20, h: 20, cx: 50, cy: 50, mod: mkMod('small'), leaf: true, depth: 1, childIds: [] },
      },
    }
    const i = buildHitTest(overlap)
    expect(i.queryPoint(50, 50)).toBe('small')
    expect(i.queryPoint(5, 5)).toBe('big')
  })
})
