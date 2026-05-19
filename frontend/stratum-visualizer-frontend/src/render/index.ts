import { GraphSnapshot, Violation } from '../types'
import { adaptSnapshot } from './adapt'
import { buildEdgeGraphics } from './edges'
import { Highlights, buildHighlights, edgeKey } from './highlights'
import { computeLayout } from './layout'
import { buildNodeSprite, NodeSprite } from './nodes'
import { createScene, Scene } from './scene'
import { sliceIfNeeded } from './sliced'

export interface RenderHandle {
  destroy: () => void
  scene: Scene
  sliced: boolean
  highlights: Highlights
}

export interface RenderOptions {
  expanded?: ReadonlySet<number>
  onAggregateClick?: (layer: number) => void
  violations?: Violation[]
}

export const renderGraph = async (
  host: HTMLElement,
  snapshot: GraphSnapshot,
  options: RenderOptions = {},
): Promise<RenderHandle> => {
  const adapted = adaptSnapshot(snapshot)
  const { sliced, graph } = sliceIfNeeded(adapted, options.expanded)
  const positioned = computeLayout(graph, { rankdir: 'LR' })
  const highlights = buildHighlights(options.violations ?? [])

  const scene = await createScene(host)

  for (const e of positioned.edges) {
    const key = edgeKey(e.from, e.to)
    scene.edgeLayer.addChild(
      buildEdgeGraphics(e, {
        hasError: highlights.errorEdges.has(key),
        hasWarning: highlights.warnEdges.has(key),
      }),
    )
  }
  for (const n of positioned.nodes) {
    const sprite: NodeSprite = buildNodeSprite(n, {
      hasError: highlights.errorModuleIds.has(n.id),
      hasWarning: highlights.warnModuleIds.has(n.id),
    })
    if (options.onAggregateClick && n.id.startsWith('__layer-')) {
      sprite.on('pointertap', () => {
        const layer = Number(n.id.slice('__layer-'.length))
        options.onAggregateClick!(layer)
      })
    }
    scene.nodeLayer.addChild(sprite)
  }

  if (positioned.bounds.width > 0 && positioned.bounds.height > 0) {
    scene.viewport.fit(true, positioned.bounds.width, positioned.bounds.height)
    scene.viewport.moveCenter(positioned.bounds.width / 2, positioned.bounds.height / 2)
  }

  return { destroy: scene.destroy, scene, sliced, highlights }
}
