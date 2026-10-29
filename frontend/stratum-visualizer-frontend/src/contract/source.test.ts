import { describe, expect, it } from 'vitest'
import { highlightedLinesOf } from './highlight'
import { rowsOf } from './source'

describe('highlightedLinesOf', () => {
  it('closes a comment span at a line break and reopens it on the next line', () => {
    const lines = highlightedLinesOf('src/a.ts', ['/* the user', '   model */ const x = 1'])
    expect(lines).toHaveLength(2)
    expect(lines?.[0]).toBe('<span class="hljs-comment">/* the user</span>')
    expect(lines?.[1].startsWith('<span class="hljs-comment">   model */</span>')).toBe(true)
  })
})

describe('rowsOf', () => {
  const source = { path: 'src/a.ts', start: 5, lines: ['a', 'b', 'c'], total: 40 }

  it.each([
    [6, [false, true, false]],
    [9, [false, false, false]],
  ])('numbers rows from the 1-based window start and marks line %i', (mark, marked) => {
    const rows = rowsOf(source, mark, null)
    expect(rows.map((r) => r.line)).toEqual([5, 6, 7])
    expect(rows.map((r) => r.isMarked)).toEqual(marked)
  })
})
