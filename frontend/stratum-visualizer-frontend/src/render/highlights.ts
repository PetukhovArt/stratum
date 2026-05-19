import { Severity, Violation } from '../types'

export interface Highlights {
  errorModuleIds: Set<string>
  warnModuleIds: Set<string>
  errorEdges: Set<string>
  warnEdges: Set<string>
  moduleViolations: Map<string, Violation[]>
}

const moduleKey = (id: number): string => `m:${id}`
export const edgeKey = (from: string, to: string): string => `${from}->${to}`

export const buildHighlights = (violations: Violation[]): Highlights => {
  const errorModuleIds = new Set<string>()
  const warnModuleIds = new Set<string>()
  const errorEdges = new Set<string>()
  const warnEdges = new Set<string>()
  const moduleViolations = new Map<string, Violation[]>()

  for (const v of violations) {
    const isError = v.severity === Severity.Error
    for (const mid of v.modules) {
      const k = moduleKey(mid)
      if (isError) errorModuleIds.add(k)
      else if (v.severity === Severity.Warning) warnModuleIds.add(k)
      const list = moduleViolations.get(k) ?? []
      list.push(v)
      moduleViolations.set(k, list)
    }
    if (v.edge) {
      const k = edgeKey(moduleKey(v.edge.from), moduleKey(v.edge.to))
      if (isError) errorEdges.add(k)
      else if (v.severity === Severity.Warning) warnEdges.add(k)
    }
  }

  return { errorModuleIds, warnModuleIds, errorEdges, warnEdges, moduleViolations }
}
