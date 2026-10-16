import { Facet } from './design'
import { Tone } from './encoding'

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

export interface RankInfo {
  name: string
  desc: string
  color: string
}

export const STAGE: Record<number, RankInfo> = {
  1: {
    name: 'file',
    desc: 'Stage 1 — file. The container is a single file.',
    color: 'var(--stage-1)',
  },
  2: {
    name: 'flat folder',
    desc: 'Stage 2 — flat folder. Files of one theme side by side, behind an entry.',
    color: 'var(--stage-2)',
  },
  3: {
    name: 'segmented',
    desc: 'Stage 3 — segmented. Files grouped into segment folders by purpose.',
    color: 'var(--stage-3)',
  },
  4: {
    name: 'composed',
    desc: 'Stage 4 — composed. Built from sub-units, each behind its own entry.',
    color: 'var(--stage-4)',
  },
}

export const PURITY: Record<number, RankInfo> = {
  1: {
    name: 'pure',
    desc: 'Purity 1 — pure. May import only purity-1 modules.',
    color: 'var(--purity-1)',
  },
  2: {
    name: 'mostly pure',
    desc: 'Purity 2 (the default) — may import modules of purity 1–2.',
    color: 'var(--purity-2)',
  },
  3: {
    name: 'impure',
    desc: 'Purity 3 — may import modules of purity 1–3.',
    color: 'var(--purity-3)',
  },
  4: {
    name: 'side effects',
    desc: 'Purity 4 — side effects. May import any module.',
    color: 'var(--purity-4)',
  },
}

export const RANKS: readonly number[] = [1, 2, 3, 4]

const UNKNOWN_RANK: RankInfo = { name: '?', desc: '', color: '#666' }

export const stageInfo = (stage: number): RankInfo => STAGE[stage] ?? UNKNOWN_RANK

export const purityInfo = (purity: number): RankInfo => PURITY[purity] ?? UNKNOWN_RANK

export const facetInfo = (facet: Facet): RankInfo & { axis: string } =>
  facet.axis === 'stage'
    ? { axis: 'Stage', ...stageInfo(facet.value) }
    : { axis: 'Purity', ...purityInfo(facet.value) }
