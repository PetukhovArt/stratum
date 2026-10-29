import { createSignal } from 'solid-js'
import { Loc } from '../lib/api'
import { nav } from '../shell/nav'

export type CodeSite = Loc & { kind: 'import' | 'call' }

export interface CodeTarget {
  name: string
  signature: string | null
  decl: Loc | null
  site: CodeSite | null
}

interface Opened {
  target: CodeTarget
  view: string
  features: string
}

const scopeOf = () => ({ view: nav().view, features: (nav().features ?? []).join('\n') })

const [opened, setOpened] = createSignal<Opened | null>(null)

/** The open target while the view and the feature choice it was opened in are still shown. */
export const codeTarget = (): CodeTarget | null => {
  const o = opened()
  if (!o) return null
  const now = scopeOf()
  return o.view === now.view && o.features === now.features ? o.target : null
}

export const openCode = (next: CodeTarget): void => {
  setOpened({ target: next, ...scopeOf() })
}

export const closeCode = (): boolean => {
  if (!codeTarget()) return false
  setOpened(null)
  return true
}
