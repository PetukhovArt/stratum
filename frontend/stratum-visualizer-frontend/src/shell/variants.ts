import { Variants } from '../views/view-def'
import { NavPatch, NavState } from './nav'

export const variantIndex = (v: Variants, state: NavState): number =>
  v.options.findIndex((o) => o.id === state[v.key])

export const variantPatch = (v: Variants, index: number): NavPatch => {
  switch (v.key) {
    case 'focusVariant':
      return { focusVariant: v.options[index].id, sel: null }
    case 'dsmLevel':
      return { dsmLevel: v.options[index].id, sel: null }
    case 'mrVariant':
      return { mrVariant: v.options[index].id, sel: null }
    case 'mapVariant':
      return { mapVariant: v.options[index].id, sel: null }
  }
}
