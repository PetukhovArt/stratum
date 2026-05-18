import { Container, Graphics, Text } from 'pixi.js'
import { PositionedNode } from './layout'

const KIND_COLORS: Record<string, number> = {
  module: 0x4a90e2,
  container: 0x2b3a55,
  'layer-aggregate': 0x8e44ad,
}

const NODE_ID_KEY = '__nodeId'

export interface NodeSprite extends Container {
  [NODE_ID_KEY]?: string
}

export const buildNodeSprite = (node: PositionedNode): NodeSprite => {
  const container: NodeSprite = new Container()
  container.position.set(node.x, node.y)
  container.pivot.set(node.width / 2, node.height / 2)
  container[NODE_ID_KEY] = node.id
  container.eventMode = 'static'
  container.cursor = 'pointer'

  const bg = new Graphics()
  bg.roundRect(0, 0, node.width, node.height, 6)
  bg.fill({ color: KIND_COLORS[node.kind] ?? 0x666666 })
  bg.stroke({ width: 1, color: 0x000000, alpha: 0.4 })
  container.addChild(bg)

  const text = new Text({
    text: shorten(node.label, 28),
    style: { fontFamily: 'monospace', fontSize: 12, fill: 0xffffff },
  })
  text.position.set(8, (node.height - text.height) / 2)
  container.addChild(text)

  return container
}

const shorten = (s: string, max: number): string => (s.length <= max ? s : '…' + s.slice(-(max - 1)))
