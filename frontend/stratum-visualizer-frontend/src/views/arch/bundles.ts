import { Model } from '../../render/model'
import { mapDataOf, UnitLink } from '../map/units'

const isSlice = (m: Model, id: number): boolean => m.containers.get(id)?.kind === 'slice'

export const crossSliceBundles = (m: Model): UnitLink[] =>
  mapDataOf(m)
    .links.filter((l) => isSlice(m, l.from) && isSlice(m, l.to))
    .sort((a, b) => b.count - a.count || a.key.localeCompare(b.key))
