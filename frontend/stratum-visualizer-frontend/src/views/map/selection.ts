import { nav } from '../../shell/nav'

export interface Pair {
  from: number
  to: number
}

export const selectedUnit = (): number | null => {
  const sel = nav().sel
  return sel?.kind === 'container' ? sel.id : null
}

export const selectedUnitPair = (): Pair | null => {
  const sel = nav().sel
  return sel?.kind === 'cell' && sel.level === 'unit' ? { from: sel.from, to: sel.to } : null
}
