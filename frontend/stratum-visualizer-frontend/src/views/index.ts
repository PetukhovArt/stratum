import { ViewId } from '../shell/nav'
import { focusView } from './focus/focus'
import { overviewView } from './overview/overview'
import { placeholderView } from './placeholder'
import { ViewDef } from './view-def'

export type { ViewDef, ViewProps } from './view-def'

export const VIEWS: readonly ViewDef[] = [
  focusView,
  placeholderView('dsm', '2', 'view.dsm'),
  placeholderView('mr', '3', 'view.mr'),
  overviewView,
]

export const viewById = (id: ViewId): ViewDef => {
  const view = VIEWS.find((v) => v.id === id)
  if (!view) throw new Error(`No view registered for '${id}'`)
  return view
}
