import { nav, set } from '../../shell/nav'

export interface Pair {
  from: number
  to: number
}

export const selectedUnit = (): number | null => {
  const sel = nav().sel
  return sel?.kind === 'container' ? sel.id : null
}

const selectedPair = (level: 'unit' | 'layer'): Pair | null => {
  const sel = nav().sel
  return sel?.kind === 'cell' && sel.level === level ? { from: sel.from, to: sel.to } : null
}

export const selectedUnitPair = (): Pair | null => selectedPair('unit')

export const selectedLayerPair = (): Pair | null => selectedPair('layer')

export const toggleExpanded = (id: number): void => {
  const expanded = nav().mapExpanded
  set({ mapExpanded: expanded.includes(id) ? expanded.filter((x) => x !== id) : [...expanded, id] })
}
