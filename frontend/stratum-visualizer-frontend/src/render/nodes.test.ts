import { describe, expect, it } from 'vitest'
import { Text } from 'pixi.js'
import { PositionedNode } from './layout'
import { buildNodeSprite } from './nodes'

const node: PositionedNode = {
  id: 'm:1',
  label: 'src/foo/bar.ts',
  layer: 1,
  kind: 'module',
  parent: null,
  x: 100,
  y: 50,
  width: 160,
  height: 40,
}

describe('buildNodeSprite', () => {
  it('positions the sprite at node coordinates', () => {
    const s = buildNodeSprite(node)
    expect(s.position.x).toBe(100)
    expect(s.position.y).toBe(50)
  })

  it('attaches node id for hit-testing', () => {
    const s = buildNodeSprite(node)
    expect((s as { __nodeId?: string }).__nodeId).toBe('m:1')
  })

  it.skip('renders a text label with the node label', () => {
    // PixiJS Text needs CanvasRenderingContext2D which happy-dom doesn't expose.
    // Covered by Playwright e2e instead.
    const s = buildNodeSprite(node)
    const label = s.children.find((c) => c instanceof Text) as Text | undefined
    expect(label?.text).toBe('src/foo/bar.ts')
  })
})
