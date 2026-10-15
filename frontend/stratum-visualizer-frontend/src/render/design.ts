export type ModuleKind = 'regular' | 'composer' | 'shared'
export type SeverityLabel = 'error' | 'warning' | 'info'

export interface DesignLayer {
  id: string
  label: string
  order: number
  hue: number
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
  modules: string[]
  isCycle: boolean
}

export interface DesignModule {
  id: string
  container: string
  parentId: string | null
  label: string
  path: string
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
  severity: SeverityLabel | null
}

export interface DesignData {
  meta: {
    project: string
    loadedAt: string
    snapshotVersion: number
  }
  layers: DesignLayer[]
  containers: DesignContainer[]
  modules: DesignModule[]
  childIndex: Record<string, string[]>
  topByContainer: Record<string, string[]>
  edges: DesignEdge[]
  violations: DesignViolation[]
  moduleById: ReadonlyMap<string, DesignModule>
  violationById: ReadonlyMap<string, DesignViolation>
  layerByContainer: ReadonlyMap<string, DesignLayer>
}

export const layerOfModule = (data: DesignData, mod: DesignModule): DesignLayer | null =>
  data.layerByContainer.get(mod.container) ?? null

export const violationsOf = (data: DesignData, mod: DesignModule): DesignViolation[] =>
  mod.violations
    .map((vid) => data.violationById.get(vid))
    .filter((v): v is DesignViolation => v !== undefined)

export const ancestorIds = (
  moduleById: ReadonlyMap<string, DesignModule>,
  id: string,
): string[] => {
  const out: string[] = []
  let cur = moduleById.get(id)?.parentId ?? null
  while (cur) {
    out.push(cur)
    cur = moduleById.get(cur)?.parentId ?? null
  }
  return out
}
