export type SeverityLabel = 'error' | 'warning' | 'info'
export type EdgeKindLabel = 'static' | 'di' | 'runtime'

export const SEVERITY_LABELS: readonly SeverityLabel[] = ['error', 'warning', 'info']

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
} as const satisfies Record<string, Tone>

export const layerColor = (hue: number, tone: Tone = LAYER_TONE.swatch): string =>
  `oklch(${tone.lightness}% ${tone.chroma} ${hue} / ${tone.alpha})`
