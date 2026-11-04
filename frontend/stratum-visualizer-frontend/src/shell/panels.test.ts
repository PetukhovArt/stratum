import { beforeEach, describe, expect, it, vi } from 'vitest'

const freshPanels = async () => {
  vi.resetModules()
  return import('./panels')
}

describe('side panels', () => {
  beforeEach(() => localStorage.clear())

  it('restores a collapsed panel and a dragged width after a reload', async () => {
    const first = await freshPanels()
    first.togglePanel('right')
    first.setPanelWidth('left', 300)

    const next = await freshPanels()
    expect(next.isPanelCollapsed('right')).toBe(true)
    expect(next.isPanelCollapsed('left')).toBe(false)
    expect(next.panelWidth('left')).toBe(300)
  })
})
