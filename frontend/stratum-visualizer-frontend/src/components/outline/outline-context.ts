import { createContext, useContext } from 'solid-js'
import { DesignData } from '../../render/design'
import { TipController } from './tree-tooltip'

export interface OutlineContextValue {
  data: DesignData
  tip: TipController
  selectedId: () => string | null
  onSelect: (id: string) => void
  toggle: (id: string) => void
  isCollapsed: (id: string) => boolean
  isModuleVisible: (id: string) => boolean
}

export const OutlineContext = createContext<OutlineContextValue>()

export const useOutline = (): OutlineContextValue => {
  const ctx = useContext(OutlineContext)
  if (!ctx) {
    throw new Error('useOutline must be used inside OutlinePanel')
  }
  return ctx
}
