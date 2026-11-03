import { SeverityLabel } from '../../render/encoding'
import { AggregateEdge, edgeKey, Model, Unit, worst } from '../../render/model'
import { Violation } from '../../types'
import { isBreak } from '../dsm/layout'

export interface UnitLink extends AggregateEdge {
  key: string
  /** Imports that carry a cross-layer or sibling finding: the matrix's rule breaks. */
  breaks: number
  breakWorst: SeverityLabel | null
}

export interface UnitInfo {
  unit: Unit
  errors: number
  inn: readonly UnitLink[]
  out: readonly UnitLink[]
}

export interface UnitData {
  units: ReadonlyMap<number, UnitInfo>
  links: readonly UnitLink[]
  linkOf: (from: number, to: number) => UnitLink | undefined
}

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

const buildUnitData = (m: Model): UnitData => {
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
      const unitIn = inn.get(unit.id) ?? []
      const unitOut = out.get(unit.id) ?? []
      return [
        unit.id,
        {
          unit,
          errors: unitFindings(m, unit).filter((v) => v.severity === 'error').length,
          inn: unitIn,
          out: unitOut,
        },
      ]
    }),
  )

  return {
    units,
    links,
    linkOf: (from, to) => byKey.get(edgeKey(from, to)),
  }
}

const cache = new WeakMap<Model, UnitData>()

export const unitDataOf = (m: Model): UnitData => {
  const cached = cache.get(m)
  if (cached) return cached
  const made = buildUnitData(m)
  cache.set(m, made)
  return made
}
