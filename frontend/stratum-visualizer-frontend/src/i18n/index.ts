import { createSignal } from 'solid-js'
import { en, MessageKey, Messages, ru } from './catalog'
import type { Plural } from './types'

export type { MessageKey }

export type Lang = 'en' | 'ru'
export const LANGS: readonly Lang[] = ['en', 'ru']

type Params = Record<string, string | number>

const STORAGE_KEY = 'stratum.lang'
const CATALOG: Record<Lang, Messages> = { en, ru }

const storedLang = (): Lang => (localStorage.getItem(STORAGE_KEY) === 'ru' ? 'ru' : 'en')

const [lang, setLangSignal] = createSignal<Lang>(storedLang())
document.documentElement.lang = lang()

export { lang }

export const setLang = (next: Lang): void => {
  setLangSignal(next)
  localStorage.setItem(STORAGE_KEY, next)
  document.documentElement.lang = next
}

const pluralRules = new Map<Lang, Intl.PluralRules>()
const numberFormats = new Map<Lang, Intl.NumberFormat>()

const cached = <T>(cache: Map<Lang, T>, l: Lang, make: () => T): T => {
  const hit = cache.get(l)
  if (hit) return hit
  const made = make()
  cache.set(l, made)
  return made
}

const pickPlural = (plural: Plural, l: Lang, n: unknown): string => {
  if (typeof n !== 'number') return plural.other
  const category = cached(pluralRules, l, () => new Intl.PluralRules(l)).select(n)
  const form = category === 'one' || category === 'few' || category === 'many' ? plural[category] : undefined
  return form ?? plural.other
}

const interpolate = (text: string, l: Lang, params?: Params): string => {
  if (!params) return text
  const format = cached(numberFormats, l, () => new Intl.NumberFormat(l))
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => {
    const value = params[name]
    if (value === undefined) return whole
    return typeof value === 'number' ? format.format(value) : value
  })
}

export const t = (key: MessageKey, params?: Params): string => {
  const l = lang()
  const message = CATALOG[l][key]
  const text = typeof message === 'string' ? message : pickPlural(message, l, params?.n)
  return interpolate(text, l, params)
}
