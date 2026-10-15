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
  childIndex: Record<string, string[]>
  topByContainer: Record<string, string[]>
  edges: DesignEdge[]
  violations: DesignViolation[]
}
