import { layerColor, LAYER_TONE, SEV_COLOR, Tone } from '../../render/encoding'
import { Model } from '../../render/model'
import { EdgeChange } from './drift'
import { worstBreak } from './review'

const GHOST_SEGMENTS = 2

export const ADDED_STROKE = 'oklch(95% 0.005 260)'
export const NEUTRAL_STROKE = 'oklch(72% 0.02 260)'
export const REMOVED_DASH = '3 4'

export const trunc = (s: string, n: number): string =>
  s.length <= n ? s : `${s.slice(0, Math.ceil(n / 2) - 1)}…${s.slice(-Math.floor(n / 2))}`

export const shortPath = (path: string): string => path.split('/').slice(-GHOST_SEGMENTS).join('/')

export const labelOf = (m: Model, id: number, path: string): string => (id < 0 ? shortPath(path) : m.label(id))

export const layerIndex = (m: Model, layer: number | null): number =>
  layer === null ? -1 : m.layers.findIndex((l) => l.id === layer)

export const layerTint = (m: Model, layer: number | null, tone: Tone = LAYER_TONE.swatch): string => {
  const index = layerIndex(m, layer)
  return index < 0 ? 'oklch(60% 0.01 260 / .5)' : layerColor(m.layerHue(index), tone)
}

export const rulesOf = (breaks: EdgeChange['breaks']): string => breaks.map(([slug]) => slug).join(' ')

export const breakColor = (breaks: EdgeChange['breaks']): string | null => {
  const sev = worstBreak(breaks)
  return sev ? SEV_COLOR[sev] : null
}
