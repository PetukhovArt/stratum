import { DesignModule } from '../render/design'
import { DEFAULT_FILTERS, Filters } from '../state'
import { globToRegex, normalizePath } from './glob'

const moduleFilterFlags = (f: Filters): boolean[] => [
  f.isViolatorsOnly,
  f.query !== '',
  f.glob !== '',
  f.stageFilter !== null,
]

export const isModuleFilterActive = (f: Filters): boolean => moduleFilterFlags(f).some(Boolean)

export const activeFilterCount = (f: Filters): number =>
  f.disabledLayers.size + f.disabledContainers.size + moduleFilterFlags(f).filter(Boolean).length

export const clearFilters = (f: Filters): Filters => ({
  ...DEFAULT_FILTERS(),
  edgeKinds: f.edgeKinds,
  connectionMode: f.connectionMode,
})

export const moduleMatcher = (f: Filters): ((mod: DesignModule) => boolean) => {
  const matchesGlob = globToRegex(f.glob)
  const query = f.query ? normalizePath(f.query.toLowerCase()) : null
  return (mod) => {
    const hasViolations = mod.violations.length > 0 || mod.descError > 0 || mod.descWarn > 0
    if (f.isViolatorsOnly && !hasViolations) return false
    if (f.stageFilter !== null && mod.stage !== f.stageFilter) return false
    if (query) {
      const pathLc = normalizePath(mod.path.toLowerCase())
      if (!mod.id.toLowerCase().includes(query) && !pathLc.includes(query)) return false
    }
    return !matchesGlob || matchesGlob(mod.path)
  }
}

export const toggled = <T>(set: ReadonlySet<T>, value: T): Set<T> => {
  const next = new Set(set)
  if (next.has(value)) {
    next.delete(value)
  } else {
    next.add(value)
  }
  return next
}
