import { layerColor, LAYER_TONE, SeverityLabel, Tone } from '../../render/encoding'
import { Model } from '../../render/model'
import { Violation } from '../../types'

interface LayerInfo {
  name: string
  hue: number
}

interface ModelPaint {
  layers: Map<number, LayerInfo>
  entries: Map<number, string[]>
}

const cache = new WeakMap<Model, ModelPaint>()

const paintOf = (m: Model): ModelPaint => {
  const cached = cache.get(m)
  if (cached) return cached
  const layers = new Map(m.layers.map((l, i) => [l.id, { name: l.name, hue: m.layerHue(i) }]))
  const entries = new Map<number, string[]>()
  for (const c of m.containers.values()) {
    if (c.entry === null) continue
    entries.set(c.entry, [...(entries.get(c.entry) ?? []), c.name])
  }
  const made = { layers, entries }
  cache.set(m, made)
  return made
}

export const layerName = (m: Model, layer: number): string => paintOf(m).layers.get(layer)?.name ?? String(layer)

export const layerTone = (m: Model, layer: number, tone: Tone = LAYER_TONE.swatch): string =>
  layerColor(paintOf(m).layers.get(layer)?.hue ?? 0, tone)

export const entryOf = (m: Model, id: number): readonly string[] => paintOf(m).entries.get(id) ?? []

export const trunc = (s: string, n: number): string =>
  s.length <= n ? s : `${s.slice(0, Math.ceil(n / 2) - 1)}…${s.slice(-Math.floor(n / 2))}`

const MIN_CURVE_DX = 40

export const curve = (x1: number, y1: number, x2: number, y2: number): string => {
  const dx = Math.max(MIN_CURVE_DX, (x2 - x1) / 2)
  return `M${x1},${y1} C${x1 + dx},${y1} ${x2 - dx},${y2} ${x2},${y2}`
}

export const rulesOf = (findings: readonly Violation[]): string =>
  [...new Set(findings.map((v) => v.rule))].sort().join(' ')

export const shortRule = (rule: string): string => rule.replace(/^stratum\//, '')

export const severityCounts = (findings: readonly Violation[]): [SeverityLabel, number][] => {
  const counts: Record<SeverityLabel, number> = { error: 0, warning: 0, info: 0 }
  for (const v of findings) {
    if (v.severity in counts) counts[v.severity as SeverityLabel]++
  }
  return (Object.entries(counts) as [SeverityLabel, number][]).filter(([, n]) => n > 0)
}
