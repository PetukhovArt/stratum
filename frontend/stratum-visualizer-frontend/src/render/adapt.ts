import { EdgeKind, GraphSnapshot } from '../types'

export interface RenderNode {
  id: string
  label: string
  layer: number
  kind: string
  parent: string | null
}

export interface RenderEdge {
  from: string
  to: string
  kind: string
}

export interface RenderGraph {
  nodes: RenderNode[]
  edges: RenderEdge[]
  layerNames: string[]
}

const moduleId = (id: number): string => `m:${id}`
const containerId = (id: number): string => `c:${id}`

export const adaptSnapshot = (snap: GraphSnapshot): RenderGraph => {
  const containerById = new Map<number, { name: string; layer: number; parent: number | null }>()
  for (const c of snap.containers) containerById.set(c.id, c)

  const nodes: RenderNode[] = []
  for (const c of snap.containers) {
    nodes.push({
      id: containerId(c.id),
      label: c.name,
      layer: c.layer,
      kind: 'container',
      parent: c.parent !== null ? containerId(c.parent) : null,
    })
  }
  for (const m of snap.modules) {
    nodes.push({
      id: moduleId(m.id),
      label: m.path,
      layer: m.layer,
      kind: 'module',
      parent: containerById.has(m.container) ? containerId(m.container) : null,
    })
  }

  const edges: RenderEdge[] = snap.edges.map((e) => ({
    from: moduleId(e.from),
    to: moduleId(e.to),
    kind: edgeKindLabel(e.kind),
  }))

  const layerNames = [...snap.layers].sort((a, b) => a.id - b.id).map((l) => l.name)
  return { nodes, edges, layerNames }
}

const edgeKindLabel = (kind: EdgeKind): string => {
  switch (kind) {
    case EdgeKind.Static:
      return 'static'
    case EdgeKind.Di:
      return 'di'
    case EdgeKind.Runtime:
      return 'runtime'
    default:
      return 'unknown'
  }
}
