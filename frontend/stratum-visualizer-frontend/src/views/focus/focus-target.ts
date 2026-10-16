import { Model } from '../../render/model'

const byFanIn = new WeakMap<Model, readonly number[]>()

export const mostImported = (m: Model): readonly number[] => {
  const cached = byFanIn.get(m)
  if (cached) return cached
  const ranked = [...m.modules.keys()].sort((a, b) => m.inn(b).length - m.inn(a).length || a - b)
  byFanIn.set(m, ranked)
  return ranked
}

export const resolveFocus = (m: Model, requested: number | null): number | null => {
  if (requested !== null && m.modules.has(requested)) return requested
  return mostImported(m)[0] ?? null
}
