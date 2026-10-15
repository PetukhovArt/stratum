import { EdgeKindLabel } from './render/design'

export interface Filters {
  disabledLayers: Set<string>
  disabledContainers: Set<string>
  edgeKinds: Set<EdgeKindLabel>
  isViolatorsOnly: boolean
  query: string
  glob: string
  stageFilter: number | null
  connectionMode: 'minimal' | 'all'
}

export const ALL_EDGE_KINDS: readonly EdgeKindLabel[] = ['static', 'di', 'runtime']

export const DEFAULT_FILTERS = (): Filters => ({
  disabledLayers: new Set(),
  disabledContainers: new Set(),
  edgeKinds: new Set(ALL_EDGE_KINDS),
  isViolatorsOnly: false,
  query: '',
  glob: '',
  stageFilter: null,
  connectionMode: 'minimal',
})

export interface Viewport {
  x: number
  y: number
  zoom: number
}

export interface HoveredMod {
  id: string
  x: number
  y: number
}

export interface HoveredEdge {
  i: number
  source: string
  target: string
  kind: EdgeKindLabel
  violation: string | null
  x: number
  y: number
}

export interface CanvasSize {
  w: number
  h: number
}
