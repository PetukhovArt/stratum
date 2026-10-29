import { activeCanvas } from '../canvas/active'
import { closeCode } from '../contract/code'
import { availableViews, viewById } from '../views'
import { back, go, nav, set } from './nav'
import { variantIndex, variantPatch } from './variants'

export interface KeymapActions {
  isPaletteOpen: () => boolean
  setPaletteOpen: (isOpen: boolean) => void
  toggleLeftPanel: () => void
  toggleRightPanel: () => void
}

const isTypingTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false
  return target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable
}

const cycleVariant = (): void => {
  const variants = viewById(nav().view).variants
  if (!variants) return
  const next = (variantIndex(variants, nav()) + 1) % variants.options.length
  set(variantPatch(variants, next))
}

const onKeyDown = (e: KeyboardEvent, actions: KeymapActions): void => {
  const hasCommandModifier = e.metaKey || e.ctrlKey
  const isTyping = isTypingTarget(e.target)

  if (hasCommandModifier && (e.key === 'p' || e.key === 'k')) {
    e.preventDefault()
    actions.setPaletteOpen(true)
    return
  }
  if (hasCommandModifier && (e.key === '[' || e.key === ']')) {
    e.preventDefault()
    if (e.key === '[') {
      actions.toggleLeftPanel()
    } else {
      actions.toggleRightPanel()
    }
    return
  }
  if (e.key === 'Escape') {
    if (actions.isPaletteOpen()) {
      actions.setPaletteOpen(false)
    } else if (!closeCode()) {
      back()
    }
    return
  }
  if (isTyping || hasCommandModifier || e.altKey) return
  if (e.key === '/') {
    e.preventDefault()
    actions.setPaletteOpen(true)
    return
  }
  if (e.key === 'f') {
    activeCanvas()?.fit()
    return
  }
  if (e.key === 'v') {
    cycleVariant()
    return
  }
  const view = availableViews().find((v) => v.key === e.key)
  if (view) go({ view: view.id })
}

export const bindKeymap = (actions: KeymapActions): (() => void) => {
  const listener = (e: KeyboardEvent) => onKeyDown(e, actions)
  window.addEventListener('keydown', listener)
  return () => window.removeEventListener('keydown', listener)
}
