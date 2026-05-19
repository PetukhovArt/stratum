import { Container, Graphics, Text } from 'pixi.js'
import { PositionedNode } from './layout'

const KIND_COLORS: Record<string, number> = {
  module: 0x4a90e2,
  container: 0x2b3a55,
  'layer-aggregate': 0x8e44ad,
}

const ERROR_FILL = 0xc0392b
const WARN_FILL = 0xd4a017

const NODE_ID_KEY = '__nodeId'

export interface NodeSprite extends Container {
  [NODE_ID_KEY]?: string
}

export interface NodeSpriteOptions {
  hasError?: boolean
  hasWarning?: boolean
}

export const buildNodeSprite = (
  node: PositionedNode,
  options: NodeSpriteOptions = {},
): NodeSprite => {
  const container: NodeSprite = new Container()
  container.position.set(node.x, node.y)
  container.pivot.set(node.width / 2, node.height / 2)
  container[NODE_ID_KEY] = node.id
  container.eventMode = 'static'
  container.cursor = 'pointer'

  const fill = options.hasError
    ? ERROR_FILL
    : options.hasWarning
      ? WARN_FILL
      : (KIND_COLORS[node.kind] ?? 0x666666)

  const bg = new Graphics()
  bg.roundRect(0, 0, node.width, node.height, 6)
  bg.fill({ color: fill })
  bg.stroke({
    width: options.hasError ? 2 : 1,
    color: options.hasError ? 0xff6b6b : 0x000000,
    alpha: options.hasError ? 1 : 0.4,
  })
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
