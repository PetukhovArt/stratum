import type { Feature } from '../lib/api'
import type { ModuleScope } from '../render/model'

export interface FeatureFilter {
  chosen: readonly string[]
  isHidingCommon: boolean
  commonLimit: number
}

export const DEFAULT_COMMON_LIMIT = 3

const countUses = (features: readonly Feature[]): Map<number, number> => {
  const uses = new Map<number, number>()
  for (const f of features) {
    for (const id of new Set(f.modules)) uses.set(id, (uses.get(id) ?? 0) + 1)
  }
  return uses
}

export const scopeOf = (features: readonly Feature[], filter: FeatureFilter): ModuleScope | null => {
  const chosenIds = new Set(filter.chosen)
  const chosen = features.filter((f) => chosenIds.has(f.id))
  if (!chosen.length && !filter.isHidingCommon) return null

  const inChosen = countUses(chosen)
  const common = new Set<number>()
  if (filter.isHidingCommon) {
    for (const [id, n] of countUses(features)) if (n > filter.commonLimit) common.add(id)
  }
  return {
    chosen: chosen.map((f) => f.id),
    keeps: (id) => !common.has(id) && (!chosen.length || inChosen.has(id)),
    isShared: (id) => (inChosen.get(id) ?? 0) >= 2,
  }
}
