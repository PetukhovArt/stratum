import { ViewId } from '../shell/nav'
import { archView } from './arch/arch'
import { benchView } from './bench/bench'
import { dsmView } from './dsm/dsm'
import { focusView } from './focus/focus'
import { mapView } from './map/map'
import { mrView } from './mr/mr'
import { setView } from './set/set'
import { ViewDef } from './view-def'

export type { ViewDef, ViewProps } from './view-def'

export const VIEWS: readonly ViewDef[] = [focusView, dsmView, mrView, mapView, setView, archView, benchView]

export const availableViews = (): readonly ViewDef[] => VIEWS.filter((v) => v.isAvailable?.() ?? true)

export const viewById = (id: ViewId): ViewDef => {
  const view = VIEWS.find((v) => v.id === id)
  if (!view) throw new Error(`No view registered for '${id}'`)
  return view
}
