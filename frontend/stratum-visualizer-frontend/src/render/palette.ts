import { SeverityLabel } from './design'

export interface SeverityPalette {
  color: string
  glow: string
  arrow: string
}

export const SEVERITY: Record<SeverityLabel, SeverityPalette> = {
  error: { color: 'var(--err)', glow: 'var(--err-glow)', arrow: 'arrow-error' },
  warning: { color: 'var(--warn)', glow: 'var(--warn-glow)', arrow: 'arrow-warning' },
  info: { color: 'var(--info)', glow: 'var(--info-glow)', arrow: 'arrow-info' },
}

export const SEVERITY_LABELS: readonly SeverityLabel[] = ['error', 'warning', 'info']

export interface Tone {
  lightness: number
  chroma: number
  alpha: number
}

export const layerColor = (hue: number, tone: Tone): string =>
  `oklch(${tone.lightness}% ${tone.chroma} ${hue} / ${tone.alpha})`

export const TONE = {
  laneFill: { lightness: 60, chroma: 0.18, alpha: 0.045 },
  laneBar: { lightness: 70, chroma: 0.2, alpha: 0.7 },
  laneLabel: { lightness: 82, chroma: 0.12, alpha: 0.95 },
  containerFill: { lightness: 65, chroma: 0.1, alpha: 0.05 },
  compoundFill: { lightness: 26, chroma: 0.08, alpha: 1 },
  leafFill: { lightness: 38, chroma: 0.1, alpha: 1 },
  moduleStroke: { lightness: 72, chroma: 0.1, alpha: 0.65 },
  composerStroke: { lightness: 78, chroma: 0.16, alpha: 0.9 },
  composerGlyph: { lightness: 84, chroma: 0.18, alpha: 0.95 },
  halo: { lightness: 72, chroma: 0.12, alpha: 0.4 },
  chipBg: { lightness: 70, chroma: 0.1, alpha: 0.2 },
  chipText: { lightness: 82, chroma: 0.1, alpha: 1 },
  dot: { lightness: 70, chroma: 0.1, alpha: 1 },
  edgeLayer: { lightness: 78, chroma: 0.1, alpha: 1 },
  minimapLane: { lightness: 60, chroma: 0.08, alpha: 0.18 },
} as const satisfies Record<string, Tone>

export const DEFAULT_HUE = 240

export const STAGE: Record<number, { name: string; desc: string; color: string }> = {
  1: {
    name: 'file',
    desc: 'Stage 1 — single file. ≤2 exports, ~50–400 LOC. The smallest unit.',
    color: 'var(--stage-1)',
  },
  2: {
    name: 'folder',
    desc: 'Stage 2 — flat folder + index. 2–6 files of one theme. Public API via index.ts.',
    color: 'var(--stage-2)',
  },
  3: {
    name: 'segments',
    desc: 'Stage 3 — segments. Folder split into model / api / ui / lib.',
    color: 'var(--stage-3)',
  },
  4: {
    name: 'composed',
    desc: 'Stage 4 — composed. composer + ≥2 sub-features + _shared. Internal structure becomes fractal.',
    color: 'var(--stage-4)',
  },
}

const UNKNOWN_STAGE = { name: '?', desc: '', color: '#666' }

export const stageInfo = (stage: number): { name: string; desc: string; color: string } =>
  STAGE[stage] ?? UNKNOWN_STAGE
