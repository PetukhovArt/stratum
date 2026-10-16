import { Group, Model } from '../../render/model'
import { Bounds } from '../../render/viewport'
import { SnapshotLayer, Violation } from '../../types'

export const BAND = {
  half: 760,
  pillW: 176,
  pillH: 22,
  gap: 6,
  innerX: 190,
  rowGap: 4,
  minH: 40,
  focusMinH: 140,
  padTop: 12,
  padBottom: 18,
  margin: 20,
  linkFocusX: 120,
} as const

export const BAND_CARD_SCALE = 0.82

export type BandSide = -1 | 1

export interface Pill {
  key: string
  side: BandSide
  group: Group
  ids: readonly number[]
  pairs: readonly (readonly [number, number])[]
  findings: readonly Violation[]
  x: number
  y: number
  w: number
  h: number
}

export interface BandLink {
  key: string
  pill: Pill
  x1: number
  y1: number
  x2: number
  y2: number
}

export interface Band {
  layer: SnapshotLayer
  index: number
  y: number
  h: number
  isImportAllowed: boolean
  isImportedByAllowed: boolean
  imports: readonly Pill[]
  importers: readonly Pill[]
}

export interface BandsLayout {
  focus: number
  bands: readonly Band[]
  links: readonly BandLink[]
  focusY: number
  bounds: Bounds
}

interface GroupEntry {
  group: Group
  ids: number[]
}

const PER_LINE = Math.max(1, Math.floor((BAND.half - BAND.innerX) / (BAND.pillW + BAND.gap)))

const groupsByLayer = (m: Model, ids: readonly number[]): Map<number, GroupEntry[]> => {
  const byKey = new Map<string, GroupEntry>()
  for (const id of new Set(ids)) {
    const group = m.groupOf(id)
    const entry = byKey.get(group.key)
    if (entry) {
      entry.ids.push(id)
    } else {
      byKey.set(group.key, { group, ids: [id] })
    }
  }
  const byLayer = new Map<number, GroupEntry[]>()
  const sorted = [...byKey.values()].sort(
    (a, b) => b.ids.length - a.ids.length || a.group.key.localeCompare(b.group.key),
  )
  for (const entry of sorted) {
    const list = byLayer.get(entry.group.layer)
    if (list) {
      list.push(entry)
    } else {
      byLayer.set(entry.group.layer, [entry])
    }
  }
  return byLayer
}

const placePills = (m: Model, focus: number, entries: readonly GroupEntry[], side: BandSide, top: number): Pill[] =>
  entries.map(({ group, ids }, k) => {
    const pairs = ids.map((id): readonly [number, number] => (side > 0 ? [focus, id] : [id, focus]))
    const offset = (k % PER_LINE) * (BAND.pillW + BAND.gap)
    return {
      key: `${side}:${group.key}`,
      side,
      group,
      ids,
      pairs,
      findings: pairs.flatMap(([a, b]) => m.findingsOnEdge(a, b)),
      x: side > 0 ? BAND.innerX + offset : -BAND.innerX - BAND.pillW - offset,
      y: top + BAND.padTop + Math.floor(k / PER_LINE) * (BAND.pillH + BAND.gap),
      w: BAND.pillW,
      h: BAND.pillH,
    }
  })

const linkOf = (pill: Pill, focusY: number): BandLink => {
  const pillY = pill.y + pill.h / 2
  return pill.side > 0
    ? { key: pill.key, pill, x1: BAND.linkFocusX, y1: focusY, x2: pill.x, y2: pillY }
    : { key: pill.key, pill, x1: pill.x + pill.w, y1: pillY, x2: -BAND.linkFocusX, y2: focusY }
}

export const layoutBands = (m: Model, focus: number): BandsLayout => {
  const module = m.modules.get(focus)
  if (!module) throw new Error(`Unknown module id ${focus}`)
  const territory = m.territoryOf(focus)
  const imports = groupsByLayer(m, m.out(focus))
  const importers = groupsByLayer(m, m.inn(focus))
  const bands: Band[] = []
  let y = 0
  m.layers.forEach((layer, index) => {
    const out = imports.get(layer.id) ?? []
    const inn = importers.get(layer.id) ?? []
    const lines = Math.max(1, Math.ceil(out.length / PER_LINE), Math.ceil(inn.length / PER_LINE))
    const minH = layer.id === module.layer ? BAND.focusMinH : BAND.minH
    const h = Math.max(minH, lines * (BAND.pillH + BAND.gap) + BAND.padBottom)
    bands.push({
      layer,
      index,
      y,
      h,
      isImportAllowed: territory.may_import_layers.includes(layer.id),
      isImportedByAllowed: territory.imported_by_layers.includes(layer.id),
      imports: placePills(m, focus, out, 1, y),
      importers: placePills(m, focus, inn, -1, y),
    })
    y += h + BAND.rowGap
  })
  const own = bands.find((b) => b.layer.id === module.layer)
  const focusY = own ? own.y + own.h / 2 : 0
  const links = bands.flatMap((b) => [...b.imports, ...b.importers].map((p) => linkOf(p, focusY)))
  const bounds = {
    x: -BAND.half - BAND.margin,
    y: -BAND.margin / 2,
    w: BAND.half * 2 + BAND.margin * 2,
    h: y + BAND.margin,
  }
  return { focus, bands, links, focusY, bounds }
}
