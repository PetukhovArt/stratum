// Shared cross-component state types — filters, tweaks, viewport.

export interface Filters {
  disabledLayers: Set<string>
  disabledContainers: Set<string>
  edgeKinds: Set<'static' | 'di' | 'runtime'>
  onlyViolators: boolean
  query: string
  glob: string
  stageFilter: number | null
  /**
   * `minimal` (default) — render only violation edges + edges adjacent to the
   * hovered/selected module. The thousands of static edges in a real codebase
   * stay hidden until the user opts in via the facet chip.
   * `all` — render every edge that passes the other filters.
   */
  connectionMode: 'minimal' | 'all'
}

export interface Tweaks {
  haloThick: boolean
  showSubmodules: boolean
  layerMode: 'swimlanes' | 'borders' | 'none'
  direction: 'TB' | 'LR'
  density: 'compact' | 'cozy' | 'roomy'
  edgeStyle: 'curve' | 'ortho' | 'line'
  colorMode: 'layer' | 'stage'
  accent: 'amber' | 'cyan' | 'magenta' | 'lime'
  showTooltip: boolean
}

export const DEFAULT_TWEAKS: Tweaks = {
  haloThick: true,
  showSubmodules: true,
  layerMode: 'swimlanes',
  direction: 'TB',
  density: 'cozy',
  edgeStyle: 'curve',
  colorMode: 'layer',
  accent: 'amber',
  showTooltip: true,
}

export const DEFAULT_FILTERS = (): Filters => ({
  disabledLayers: new Set(),
  disabledContainers: new Set(),
  edgeKinds: new Set(['static', 'di', 'runtime']),
  onlyViolators: false,
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
  /**
   * True when the hovered module renders below the on-screen pixel threshold
   * needed to read its inline label (set by the WebGL renderer; the SVG
   * renderer leaves it undefined). Used by `HoverTooltip` to surface an
   * "Open card" affordance for modules the user can barely see.
   */
  tiny?: boolean
}

export interface HoveredEdge {
  i: number
  source: string
  target: string
  kind: 'static' | 'di' | 'runtime'
  violation: string | null
  x: number
  y: number
}

export interface StageSize {
  w: number
  h: number
}
