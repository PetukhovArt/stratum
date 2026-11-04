import { createSignal } from 'solid-js'
import { Feature, fetchFeatures } from '../../lib/api'
import { SeverityLabel } from '../../render/encoding'
import { Model } from '../../render/model'
import { CROSS_LAYER_RULE, CYCLE_RULE, SIBLING_RULE } from '../../types'
import type { ArchPath } from './path'

export type BoxKind = 'feature' | 'layer' | 'unit'

export interface Box {
  key: string
  name: string
  kind: BoxKind
  layer: number | null
  units: readonly number[]
  modules: number
}

export interface BoxLink {
  key: string
  from: string
  to: string
  count: number
  findings: number
  worst: SeverityLabel | null
  breaks: boolean
}

export interface Boxes {
  boxes: readonly Box[]
  links: readonly BoxLink[]
  boxOfUnit: ReadonlyMap<number, string>
}

const SEVERITY_RANK: Readonly<Record<SeverityLabel, number>> = { info: 0, warning: 1, error: 2 }

const worse = (a: SeverityLabel | null, b: SeverityLabel | null): SeverityLabel | null => {
  if (a === null) return b
  if (b === null) return a
  return SEVERITY_RANK[a] >= SEVERITY_RANK[b] ? a : b
}

const dirOf = (path: string): string => path.slice(0, Math.max(0, path.lastIndexOf('/')))

const commonDir = (paths: readonly string[]): string => {
  if (!paths.length) return ''
  const split = paths.map((p) => dirOf(p).split('/'))
  const first = split[0]
  let n = 0
  while (n < first.length && split.every((s) => s[n] === first[n])) n++
  return first.slice(0, n).join('/')
}

const DIRECTION_RULES: ReadonlySet<string> = new Set([CROSS_LAYER_RULE, SIBLING_RULE, CYCLE_RULE])

const isWithin = (path: string, dir: string): boolean => path === dir || path.startsWith(`${dir}/`)

const linksOf = (m: Model, boxOfUnit: ReadonlyMap<number, string>): BoxLink[] => {
  const byKey = new Map<string, BoxLink>()
  for (const e of m.unitEdges) {
    const from = boxOfUnit.get(e.from)
    const to = boxOfUnit.get(e.to)
    if (!from || !to || from === to) continue
    const key = `${from}→${to}`
    const hit = byKey.get(key)
    const breaks = e.rules.some((r) => DIRECTION_RULES.has(r))
    if (hit) {
      hit.count += e.count
      hit.findings += e.findings
      hit.worst = worse(hit.worst, e.worst)
      hit.breaks ||= breaks
    } else {
      byKey.set(key, { key, from, to, count: e.count, findings: e.findings, worst: e.worst, breaks })
    }
  }
  return [...byKey.values()].sort((a, b) => b.count - a.count)
}

type Entry = readonly [key: string, box: Omit<Box, 'modules' | 'units'>, units: readonly number[]]

const boxesOf = (m: Model, entries: readonly Entry[]): Boxes => {
  const boxOfUnit = new Map<number, string>()
  const sizeOf = new Map(m.units.map((u) => [u.id, u.modules.length]))
  const boxes = entries.flatMap(([key, box, units]): Box[] => {
    if (!units.length) return []
    for (const u of units) boxOfUnit.set(u, key)
    return [{ ...box, units, modules: units.reduce((n, id) => n + (sizeOf.get(id) ?? 0), 0) }]
  })
  return { boxes, links: linksOf(m, boxOfUnit), boxOfUnit }
}

const topLayerOf = (m: Model, unitId: number): { name: string; layer: number | null } => {
  const unit = m.units.find((u) => u.id === unitId)
  const first = unit?.modules[0]
  const outer = first === undefined ? undefined : m.chain(first)[0]
  return { name: outer?.name ?? unit?.name ?? String(unitId), layer: outer?.layer ?? unit?.layer ?? null }
}

export const featureBoxes = (m: Model, features: readonly Feature[]): Boxes => {
  const layers = [...m.containers.values()].filter((c) => c.parent === null)
  const enclosing = (dir: string) =>
    layers
      .filter((c) => c.path !== dir && isWithin(dir, c.path))
      .reduce<(typeof layers)[number] | null>((inner, c) => (inner && inner.path.length >= c.path.length ? inner : c), null)
  const taken = new Set<number>()
  const entries: Entry[] = []
  for (const f of features) {
    const dir = commonDir(f.entry.flatMap((id) => m.modules.get(id)?.path ?? []))
    const outer = dir ? enclosing(dir) : null
    if (!dir || (!outer && layers.some((c) => c.path === dir))) continue
    const units = m.units.filter((u) => !taken.has(u.id) && isWithin(u.path, dir)).map((u) => u.id)
    for (const u of units) taken.add(u)
    const key = `feature:${f.id}`
    entries.push([key, { key, name: f.name, kind: 'feature', layer: outer?.layer ?? null }, units])
  }
  const byLayer = new Map<string, { layer: number | null; units: number[] }>()
  for (const u of m.units) {
    if (taken.has(u.id)) continue
    const top = topLayerOf(m, u.id)
    const hit = byLayer.get(top.name)
    if (hit) hit.units.push(u.id)
    else byLayer.set(top.name, { layer: top.layer, units: [u.id] })
  }
  for (const [name, { layer, units }] of byLayer) {
    entries.push([`layer:${name}`, { key: `layer:${name}`, name, kind: 'layer', layer }, units])
  }
  return boxesOf(m, entries)
}

export const unitBoxes = (m: Model, units: readonly number[]): Boxes =>
  boxesOf(
    m,
    units.map((id): Entry => {
      const key = `unit:${id}`
      return [key, { key, name: m.unitLabel(id), kind: 'unit', layer: m.units.find((u) => u.id === id)?.layer ?? null }, [id]]
    }),
  )

const [features, setFeatures] = createSignal<readonly Feature[] | null>(null)
let isRequested = false

export const archFeatures = (): readonly Feature[] | null => {
  if (!isRequested) {
    isRequested = true
    fetchFeatures().then(
      (index) => setFeatures(index.features),
      () => setFeatures([]),
    )
  }
  return features()
}

export const containerBoxes = (m: Model): Boxes => featureBoxes(m, archFeatures() ?? [])

export const boxesAt = (m: Model, path: ArchPath): Boxes => {
  const top = containerBoxes(m)
  if (path.level !== 'group') return top
  return unitBoxes(m, top.boxes.find((b) => b.key === path.group)?.units ?? [])
}

export const linkOf = (data: Boxes, from: string, to: string): BoxLink | null =>
  data.links.find((l) => l.from === from && l.to === to) ?? null
