import { createRoot } from 'solid-js'
import { beforeEach, describe, expect, it } from 'vitest'
import { savedPositions, SavedPositions } from './positions'

const open = (project: string, view: string): SavedPositions => createRoot(() => savedPositions(() => project, view))

describe('savedPositions', () => {
  beforeEach(() => localStorage.clear())

  it('reads back after a reload what a drag saved, merged with earlier moves', () => {
    open('shop', 'map').save(new Map([['src/a', { x: 10.04, y: -3 }]]))
    open('shop', 'map').save(new Map([['src/b', { x: 1, y: 2 }]]))
    expect([...open('shop', 'map').at()]).toEqual([
      ['src/a', { x: 10, y: -3 }],
      ['src/b', { x: 1, y: 2 }],
    ])
  })

  it('keeps each project and view apart', () => {
    open('shop', 'map').save(new Map([['src/a', { x: 1, y: 1 }]]))
    expect(open('shop', 'arch').at().size).toBe(0)
    expect(open('blog', 'map').at().size).toBe(0)
  })

  it('forgets every move on reset, also after a reload', () => {
    const saved = open('shop', 'map')
    saved.save(new Map([['src/a', { x: 1, y: 1 }]]))
    saved.reset()
    expect(saved.at().size).toBe(0)
    expect(open('shop', 'map').at().size).toBe(0)
  })

  it('ignores a stored value it cannot read', () => {
    localStorage.setItem('stratum.positions:shop:map', '{"src/a":[1,"x"],"src/b":[3,4]')
    expect(open('shop', 'map').at().size).toBe(0)
    localStorage.setItem('stratum.positions:shop:map', '{"src/a":[1,"x"],"src/b":[3,4]}')
    expect([...open('shop', 'map').at()]).toEqual([['src/b', { x: 3, y: 4 }]])
  })
})
