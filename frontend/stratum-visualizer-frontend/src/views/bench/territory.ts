import { edgeKey } from '../../render/model'
import { Violation } from '../../types'

export type DrawnPairs = ReadonlyMap<string, readonly (readonly [from: number, to: number])[]>

export interface RuleAgreement {
  rule: string
  shownAndLinted: number
  shownNotLinted: number
  lintedNotShown: number
}

export interface TerritoryAgreement {
  rules: RuleAgreement[]
  precision: number | null
  recall: number | null
  samples: string[]
}

const MISS_SAMPLES = 5


const ratio = (part: number, whole: number): number | null => (whole ? part / whole : null)

const sum = (rules: readonly RuleAgreement[], pick: (r: RuleAgreement) => number): number =>
  rules.reduce((n, r) => n + pick(r), 0)

export const drawnPairsOf = (items: readonly { key: string; pairs: readonly (readonly [number, number])[] }[]): DrawnPairs =>
  new Map(items.map((i) => [i.key, i.pairs]))

export class TerritoryTally {
  private readonly lintRules = new Map<string, Set<string>>()
  private readonly seen = new Set<string>()
  private readonly byRule = new Map<string, RuleAgreement>()
  private readonly samples: string[] = []

  constructor(violations: readonly Violation[]) {
    for (const v of violations) {
      if (!v.edge) continue
      const key = edgeKey(v.edge.from, v.edge.to)
      const rules = this.lintRules.get(key) ?? new Set<string>()
      rules.add(v.rule)
      this.lintRules.set(key, rules)
    }
  }

  record(scene: string, drawn: DrawnPairs, stage: Element | null): void {
    const shown = new Map<string, Set<string>>()
    for (const el of stage?.querySelectorAll('[data-edge]') ?? []) {
      if (el.closest('.dsm, .map')) continue
      const rules = (el.getAttribute('data-rule') ?? '').split(' ').filter(Boolean)
      shown.set(el.getAttribute('data-edge') ?? '', new Set(rules))
    }
    for (const key of new Set([...drawn.keys(), ...shown.keys()])) {
      const pairs = drawn.get(key) ?? []
      const linted = new Set(pairs.flatMap(([a, b]) => [...(this.lintRules.get(edgeKey(a, b)) ?? [])]))
      const onEdge = shown.get(key) ?? new Set<string>()
      for (const rule of new Set([...linted, ...onEdge])) {
        this.count(`${scene} ${key}`, rule, onEdge.has(rule), linted.has(rule))
      }
    }
  }

  result(): TerritoryAgreement {
    const rules = [...this.byRule.values()].sort((a, b) => a.rule.localeCompare(b.rule))
    const agreed = sum(rules, (r) => r.shownAndLinted)
    return {
      rules,
      precision: ratio(agreed, agreed + sum(rules, (r) => r.shownNotLinted)),
      recall: ratio(agreed, agreed + sum(rules, (r) => r.lintedNotShown)),
      samples: this.samples,
    }
  }

  private count(edge: string, rule: string, isShown: boolean, isLinted: boolean): void {
    const id = `${edge}|${rule}`
    if (this.seen.has(id)) return
    this.seen.add(id)
    const agreement = this.byRule.get(rule) ?? { rule, shownAndLinted: 0, shownNotLinted: 0, lintedNotShown: 0 }
    this.byRule.set(rule, agreement)
    if (isShown && isLinted) {
      agreement.shownAndLinted++
      return
    }
    if (isShown) {
      agreement.shownNotLinted++
    } else {
      agreement.lintedNotShown++
    }
    if (this.samples.length < MISS_SAMPLES) {
      this.samples.push(`${isShown ? 'not linted' : 'not shown'}: ${rule} on ${edge}`)
    }
  }
}
