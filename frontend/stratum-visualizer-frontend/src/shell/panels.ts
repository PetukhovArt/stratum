import { createSignal } from 'solid-js'

export type PanelSide = 'left' | 'right'

const STORAGE_KEY = 'stratum.panels'

const PANEL = {
  leftWidth: 264,
  rightWidth: 340,
  minWidth: 200,
  maxWidth: 560,
} as const

interface PanelState {
  isCollapsed: boolean
  width: number
}

type Panels = Record<PanelSide, PanelState>

const DEFAULTS: Panels = {
  left: { isCollapsed: false, width: PANEL.leftWidth },
  right: { isCollapsed: false, width: PANEL.rightWidth },
}

export const clampWidth = (width: number): number => Math.max(PANEL.minWidth, Math.min(PANEL.maxWidth, width))

const readSide = (raw: unknown, fallback: PanelState): PanelState => {
  if (!raw || typeof raw !== 'object') return fallback
  const { isCollapsed, width } = raw as Partial<PanelState>
  return {
    isCollapsed: typeof isCollapsed === 'boolean' ? isCollapsed : fallback.isCollapsed,
    width: typeof width === 'number' && Number.isFinite(width) ? clampWidth(width) : fallback.width,
  }
}

const stored = (): Panels => {
  let raw: Partial<Record<PanelSide, unknown>> = {}
  try {
    raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}') ?? {}
  } catch {
    return DEFAULTS
  }
  return { left: readSide(raw.left, DEFAULTS.left), right: readSide(raw.right, DEFAULTS.right) }
}

const [panels, setPanels] = createSignal<Panels>(stored())

const update = (side: PanelSide, patch: Partial<PanelState>): void => {
  const next = { ...panels(), [side]: { ...panels()[side], ...patch } }
  setPanels(next)
  localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
}

export const isPanelCollapsed = (side: PanelSide): boolean => panels()[side].isCollapsed
export const panelWidth = (side: PanelSide): number => panels()[side].width
export const togglePanel = (side: PanelSide): void => update(side, { isCollapsed: !isPanelCollapsed(side) })
export const setPanelWidth = (side: PanelSide, width: number): void => update(side, { width: clampWidth(width) })
