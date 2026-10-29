import { beforeEach, describe, expect, it, vi } from 'vitest'

const freshNav = async () => {
  vi.resetModules()
  return import('./nav')
}

describe('nav feature filter', () => {
  beforeEach(() => localStorage.clear())

  it('starts with no choice so the server selection applies', async () => {
    const { nav } = await freshNav()
    expect(nav()).toMatchObject({ features: null, isHidingCommon: false, commonLimit: 3 })
  })

  it('restores the chosen features, an emptied choice and hide common after a reload', async () => {
    const first = await freshNav()
    first.go({ features: ['cart', 'search'], isHidingCommon: true, commonLimit: 5 })
    expect((await freshNav()).nav()).toMatchObject({
      features: ['cart', 'search'],
      isHidingCommon: true,
      commonLimit: 5,
    })

    ;(await freshNav()).go({ features: [] })
    expect((await freshNav()).nav().features).toEqual([])
  })
})
