import { RenderEdge, RenderGraph, RenderNode } from './adapt'

export const SLICE_THRESHOLD = 500

export interface SliceResult {
  sliced: boolean
  graph: RenderGraph
}

const aggregateId = (layer: number): string => `__layer-${layer}`

export const sliceIfNeeded = (
  graph: RenderGraph,
  expanded: ReadonlySet<number> = new Set(),
): SliceResult => {
  if (graph.nodes.length <= SLICE_THRESHOLD) {
    return { sliced: false, graph }
  }

  const isShown = (layer: number): boolean => expanded.has(layer)
  const kept: RenderNode[] = graph.nodes.filter((n) => isShown(n.layer))

  const countsByLayer = new Map<number, number>()
  for (const n of graph.nodes) {
    if (!isShown(n.layer)) {
      countsByLayer.set(n.layer, (countsByLayer.get(n.layer) ?? 0) + 1)
    }
  }

  graph.layerNames.forEach((name, idx) => {
    if (!expanded.has(idx)) {
      kept.push({
        id: aggregateId(idx),
        label: `${name} (${countsByLayer.get(idx) ?? 0} modules — click to expand)`,
        layer: idx,
        kind: 'layer-aggregate',
        parent: null,
      })
    }
  })

  const nodeLayer = new Map<string, number>()
  for (const n of graph.nodes) nodeLayer.set(n.id, n.layer)

  const seen = new Set<string>()
  const edges: RenderEdge[] = []
  for (const e of graph.edges) {
    const fromLayer = nodeLayer.get(e.from)
    const toLayer = nodeLayer.get(e.to)
    if (fromLayer === undefined || toLayer === undefined) continue
    const from = isShown(fromLayer) ? e.from : aggregateId(fromLayer)
    const to = isShown(toLayer) ? e.to : aggregateId(toLayer)
    const key = `${from}->${to}:${e.kind}`
    if (seen.has(key)) continue
    seen.add(key)
    edges.push({ from, to, kind: e.kind })
  }

  return { sliced: true, graph: { nodes: kept, edges, layerNames: graph.layerNames } }
}
