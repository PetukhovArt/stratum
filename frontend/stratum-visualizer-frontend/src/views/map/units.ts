import { SeverityLabel } from '../../render/encoding'
import { AggregateEdge, edgeKey, Model, Unit, worst } from '../../render/model'
import { MapGate } from '../../shell/nav'
import { Violation } from '../../types'
import { isBreak } from '../dsm/layout'

export const TOP_IMPORTS = 12

export interface UnitLink extends AggregateEdge {
  key: string
  /** Imports that carry a cross-layer or sibling finding: the matrix's rule breaks. */
  breaks: number
  breakWorst: SeverityLabel | null
}

export interface LayerLink extends AggregateEdge {
  key: string
  isDeclared: boolean
  unitPairs: number
}

export interface UnitInfo {
  unit: Unit
  fanIn: number
  fanOut: number
  findings: readonly Violation[]
  errors: number
  inn: readonly UnitLink[]
  out: readonly UnitLink[]
}

export interface MapData {
  units: ReadonlyMap<number, UnitInfo>
  links: readonly UnitLink[]
  linkOf: (from: number, to: number) => UnitLink | undefined
  layerLinks: readonly LayerLink[]
  layerLinkOf: (from: number, to: number) => LayerLink | undefined
  breakLinks: number
}


export const isGated = (link: UnitLink, gate: MapGate): boolean => {
  if (gate === 'rules') return link.breaks > 0
  if (gate === 'top') return link.count >= TOP_IMPORTS
  return true
}

export const byFanIn = (a: UnitInfo, b: UnitInfo): number => b.fanIn - a.fanIn || a.unit.id - b.unit.id

const push = <K, V>(map: Map<K, V[]>, key: K, ...values: V[]): void => {
  const list = map.get(key)
  if (list) {
    list.push(...values)
  } else {
    map.set(key, values)
  }
}

const unitBreaks = (m: Model): Map<string, Violation[]> => {
  const byLink = new Map<string, Violation[]>()
  for (const e of m.edges) {
    const from = m.unitOf(e.from)
    const to = m.unitOf(e.to)
    if (from === to) continue
    const breaking = m.findingsOnEdge(e.from, e.to).filter(isBreak)
    if (!breaking.length) continue
    push(byLink, edgeKey(from, to), ...breaking)
  }
  return byLink
}

const unitFindings = (m: Model, unit: Unit): Violation[] => [...new Set(unit.modules.flatMap((id) => m.findingsOf(id)))]

const breakingImports = (findings: readonly Violation[]): number =>
  new Set(findings.map((v) => (v.edge ? edgeKey(v.edge.from, v.edge.to) : ''))).size

const buildMapData = (m: Model): MapData => {
  const breaksByLink = unitBreaks(m)
  const links: UnitLink[] = m.unitEdges.map((e) => {
    const key = edgeKey(e.from, e.to)
    const breaking = breaksByLink.get(key) ?? []
    return { ...e, key, breaks: breakingImports(breaking), breakWorst: worst(breaking) }
  })
  const byKey = new Map(links.map((l) => [l.key, l]))

  const inn = new Map<number, UnitLink[]>()
  const out = new Map<number, UnitLink[]>()
  for (const l of links) {
    push(inn, l.to, l)
    push(out, l.from, l)
  }
  const units = new Map(
    m.units.map((unit): [number, UnitInfo] => {
      const findings = unitFindings(m, unit)
      const unitIn = inn.get(unit.id) ?? []
      const unitOut = out.get(unit.id) ?? []
      return [
        unit.id,
        {
          unit,
          fanIn: unitIn.length,
          fanOut: unitOut.length,
          findings,
          errors: findings.filter((v) => v.severity === 'error').length,
          inn: unitIn,
          out: unitOut,
        },
      ]
    }),
  )

  const layerOfUnit = new Map(m.units.map((u) => [u.id, u.layer]))
  const pairsByLayer = new Map<string, number>()
  for (const l of links) {
    const key = edgeKey(layerOfUnit.get(l.from)!, layerOfUnit.get(l.to)!)
    pairsByLayer.set(key, (pairsByLayer.get(key) ?? 0) + 1)
  }
  const dependsOn = new Map(m.layers.map((l) => [l.id, new Set(l.depends_on)]))
  const layerLinks: LayerLink[] = m.layerEdges.map((e) => {
    const key = edgeKey(e.from, e.to)
    return {
      ...e,
      key,
      isDeclared: dependsOn.get(e.from)?.has(e.to) ?? false,
      unitPairs: pairsByLayer.get(key) ?? 0,
    }
  })
  const layerByKey = new Map(layerLinks.map((l) => [l.key, l]))

  return {
    units,
    links,
    linkOf: (from, to) => byKey.get(edgeKey(from, to)),
    layerLinks,
    layerLinkOf: (from, to) => layerByKey.get(edgeKey(from, to)),
    breakLinks: links.filter((l) => l.breaks > 0).length,
  }
}

const cache = new WeakMap<Model, MapData>()

export const mapDataOf = (m: Model): MapData => {
  const cached = cache.get(m)
  if (cached) return cached
  const made = buildMapData(m)
  cache.set(m, made)
  return made
}
