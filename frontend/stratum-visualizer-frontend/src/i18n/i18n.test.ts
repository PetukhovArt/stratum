import { afterEach, describe, expect, it } from 'vitest'
import { setLang, t } from './index'

describe('t', () => {
  afterEach(() => setLang('en'))

  it.each([
    [1, '1 модуль'],
    [2, '2 модуля'],
    [5, '5 модулей'],
    [11, '11 модулей'],
    [21, '21 модуль'],
  ])('picks the Russian plural form for %i modules', (n, expected) => {
    setLang('ru')
    expect(t('shell.stats.modules', { n })).toBe(expected)
  })
})
