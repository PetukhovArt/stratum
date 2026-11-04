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

describe('nav architecture level', () => {
  beforeEach(() => localStorage.clear())

  it('restores the level and path after a reload, and the top level from a stored path it cannot read', async () => {
    ;(await freshNav()).go({ view: 'arch', arch: { level: 'code', container: 11, module: 4 } })
    expect((await freshNav()).nav()).toMatchObject({ view: 'arch', arch: { level: 'code', container: 11, module: 4 } })

    localStorage.setItem('stratum.nav', JSON.stringify({ view: 'arch', arch: { level: 'code', container: 11 } }))
    expect((await freshNav()).nav().arch).toEqual({ level: 'system' })
  })
})
