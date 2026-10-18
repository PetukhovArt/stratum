import { Severity } from '../types'

export type SeverityLabel = 'error' | 'warning' | 'info'
export type EdgeKindLabel = 'static' | 'di' | 'runtime'

export const SEVERITY_LABELS: readonly SeverityLabel[] = ['error', 'warning', 'info']

export const SEVERITY_RANK: Record<`${Severity}`, number> = { off: 0, info: 1, warning: 2, error: 3 }

export const SEV_COLOR: Record<SeverityLabel, string> = {
  error: 'oklch(64% 0.18 25)',
  warning: 'oklch(78% 0.14 70)',
  info: 'oklch(70% 0.10 230)',
}

export const EDGE_DASH: Record<EdgeKindLabel, string | undefined> = {
  static: undefined,
  runtime: '1.5 3',
  di: '6 3',
}

const LAYER_HUE_START = 200
const FULL_TURN = 360

export const layerHue = (index: number, layerCount: number): number =>
  Math.round((LAYER_HUE_START + (index * FULL_TURN) / Math.max(layerCount, 1)) % FULL_TURN)

export interface Tone {
  lightness: number
  chroma: number
  alpha: number
}

export const LAYER_TONE = {
  swatch: { lightness: 72, chroma: 0.09, alpha: 1 },
  label: { lightness: 82, chroma: 0.08, alpha: 1 },
  tint: { lightness: 34, chroma: 0.04, alpha: 0.55 },
  tintEdge: { lightness: 70, chroma: 0.08, alpha: 0.4 },
  tintEdgeStrong: { lightness: 70, chroma: 0.08, alpha: 0.8 },
  pill: { lightness: 32, chroma: 0.05, alpha: 0.95 },
  band: { lightness: 26, chroma: 0.04, alpha: 0.7 },
  bandOff: { lightness: 24, chroma: 0.03, alpha: 0.2 },
  card: { lightness: 78, chroma: 0.1, alpha: 0.9 },
  cardHalo: { lightness: 80, chroma: 0.1, alpha: 0.35 },
} as const satisfies Record<string, Tone>

export const CANVAS_DEFS = {
  arrowId: 'cv-arrow',
  hatchId: 'cv-hatch',
  arrow: 'url(#cv-arrow)',
  hatch: 'url(#cv-hatch)',
} as const

export const EDGE_NEUTRAL = 'oklch(72% 0.02 260)'
export const HATCH_STROKE = 'oklch(60% 0.01 260 / .28)'

export const layerColor = (hue: number, tone: Tone = LAYER_TONE.swatch): string =>
  `oklch(${tone.lightness}% ${tone.chroma} ${hue} / ${tone.alpha})`
