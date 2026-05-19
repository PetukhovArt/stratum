// Design data shape — what the layout and components consume.
// Produced by `adapt(GraphSnapshot, Violation[])` and structurally equivalent
// to the Claude Design handoff's STRATUM_DATA.

export type ModuleKind = 'regular' | 'composer' | 'shared'
export type SeverityLabel = 'error' | 'warning' | 'info'

export interface DesignLayer {
  id: string
  label: string
  order: number
  hue: number
  desc: string
  modules: number
  errors: number
  warnings: number
}

export interface DesignContainer {
  id: string
  layer: string
  label: string
  errors: number
  warnings: number
  moduleCount: number
  stage: number
}

export interface DesignViolation {
  id: string
  rule: string
  severity: SeverityLabel
  message: string
  ruleDoc: string
}

export interface DesignModule {
  id: string
  container: string
  parentId: string | null
  label: string
  path: string
  loc: number
  stage: number
  kind: ModuleKind
  depth: number
  hasChildren: boolean
  violations: string[]
  severity: SeverityLabel | null
  descError: number
  descWarn: number
  descInfo: number
}

export type EdgeKindLabel = 'static' | 'di' | 'runtime'

export interface DesignEdge {
  source: string
  target: string
  kind: EdgeKindLabel
  violation: string | null
}

export interface DesignData {
  meta: {
    project: string
    generatedAt: string
    snapshotVersion: number
  }
  layers: DesignLayer[]
  containers: DesignContainer[]
  modules: DesignModule[]
  /** parent module id → child module ids */
  childIndex: Record<string, string[]>
  /** container id → top-level (parentId === null) module ids */
  topByContainer: Record<string, string[]>
  edges: DesignEdge[]
  violations: DesignViolation[]
}
