import dagre from '@dagrejs/dagre'
import { RenderEdge, RenderGraph, RenderNode } from './adapt'

export interface LayoutOptions {
  rankdir: 'LR' | 'TB'
  nodeSep?: number
  rankSep?: number
  nodeWidth?: number
  nodeHeight?: number
}

export interface PositionedNode extends RenderNode {
  x: number
  y: number
  width: number
  height: number
}

export interface PositionedEdge extends RenderEdge {
  points: Array<{ x: number; y: number }>
}

export interface PositionedGraph {
  nodes: PositionedNode[]
  edges: PositionedEdge[]
  bounds: { width: number; height: number }
}

export const computeLayout = (graph: RenderGraph, options: LayoutOptions): PositionedGraph => {
  const g = new dagre.graphlib.Graph({ compound: true })
  g.setGraph({
    rankdir: options.rankdir,
    nodesep: options.nodeSep ?? 40,
    ranksep: options.rankSep ?? 80,
    marginx: 20,
    marginy: 20,
  })
  g.setDefaultEdgeLabel(() => ({}))

  const w = options.nodeWidth ?? 180
  const h = options.nodeHeight ?? 36

  for (const node of graph.nodes) {
    g.setNode(node.id, { width: w, height: h, label: node.label })
  }
  for (const node of graph.nodes) {
    if (node.parent && graph.nodes.some((n) => n.id === node.parent)) {
      g.setParent(node.id, node.parent)
    }
  }
  for (const edge of graph.edges) {
    if (g.hasNode(edge.from) && g.hasNode(edge.to)) {
      g.setEdge(edge.from, edge.to)
    }
  }

  dagre.layout(g)

  const nodes: PositionedNode[] = graph.nodes.map((n) => {
    const laid = g.node(n.id)
    return {
      ...n,
      x: laid.x,
      y: laid.y,
      width: laid.width,
      height: laid.height,
    }
  })

  const edges: PositionedEdge[] = graph.edges
    .filter((e) => g.hasNode(e.from) && g.hasNode(e.to))
    .map((e) => {
      const laid = g.edge(e.from, e.to)
      return { ...e, points: laid?.points ?? [] }
    })

  const graphLabel = g.graph()
  return {
    nodes,
    edges,
    bounds: { width: graphLabel.width ?? 0, height: graphLabel.height ?? 0 },
  }
}
