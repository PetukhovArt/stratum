import { describe, expect, it } from 'vitest'
import type { Feature } from '../lib/api'
import { scopeOf } from './scope'

const feature = (id: string, modules: number[]): Feature => ({ id, name: id, entry: modules.slice(0, 1), modules })

const FEATURES = [
  feature('cart', [1, 2, 9, 10]),
  feature('search', [3, 2, 9, 10]),
  feature('profile', [4, 9, 10]),
  feature('admin', [5, 10]),
]
const IDS = [1, 2, 3, 4, 5, 9, 10, 11]

const kept = (chosen: string[], isHidingCommon = false, commonLimit = 3) => {
  const scope = scopeOf(FEATURES, { chosen, isHidingCommon, commonLimit })
  return scope && { kept: IDS.filter(scope.keeps), shared: IDS.filter(scope.isShared) }
}

describe('scopeOf', () => {
  it('narrows nothing without chosen features or hiding common', () => {
    expect(kept([])).toBeNull()
    expect(kept(['nope'])).toBeNull()
  })

  it('keeps the union of the chosen features and marks shared only modules two chosen ones use', () => {
    expect(kept(['cart', 'search'])).toEqual({ kept: [1, 2, 3, 9, 10], shared: [2, 9, 10] })
    expect(kept(['cart'])).toEqual({ kept: [1, 2, 9, 10], shared: [] })
  })

  it('hides modules more than k of all features use, chosen or not', () => {
    expect(kept(['cart', 'search'], true, 3)?.kept).toEqual([1, 2, 3, 9])
    expect(kept(['cart', 'search'], true, 2)?.kept).toEqual([1, 2, 3])
  })

  it('hides common modules from the whole graph when no feature is chosen', () => {
    expect(kept([], true, 2)?.kept).toEqual([1, 2, 3, 4, 5, 11])
  })
})
