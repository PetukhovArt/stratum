export interface KeymapActions {
  isPaletteOpen: () => boolean
  setPaletteOpen: (isOpen: boolean) => void
  dismiss: () => void
  fit: () => void
  toggleLeftPanel: () => void
  toggleRightPanel: () => void
}

const isTypingTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false
  return target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable
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
    } else {
      actions.dismiss()
    }
    return
  }
  if (isTyping || hasCommandModifier) return
  if (e.key === '/') {
    e.preventDefault()
    actions.setPaletteOpen(true)
  }
  if (e.key === 'f') {
    actions.fit()
  }
}

export const bindKeymap = (actions: KeymapActions): (() => void) => {
  const listener = (e: KeyboardEvent) => onKeyDown(e, actions)
  window.addEventListener('keydown', listener)
  return () => window.removeEventListener('keydown', listener)
}
