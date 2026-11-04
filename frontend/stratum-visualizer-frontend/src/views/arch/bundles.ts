import { Model } from '../../render/model'
import { unitDataOf, UnitLink } from './units'

const isSlice = (m: Model, id: number): boolean => m.containers.get(id)?.kind === 'slice'

export const crossSliceBundles = (m: Model): UnitLink[] =>
  unitDataOf(m)
    .links.filter((l) => isSlice(m, l.from) && isSlice(m, l.to))
    .sort((a, b) => b.count - a.count || a.key.localeCompare(b.key))
