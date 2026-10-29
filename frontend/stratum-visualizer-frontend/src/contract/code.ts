import { createSignal } from 'solid-js'
import { Loc } from '../lib/api'

export type CodeSite = Loc & { kind: 'import' | 'call' }

export interface CodeTarget {
  name: string
  signature: string | null
  decl: Loc | null
  site: CodeSite | null
}

const [target, setTarget] = createSignal<CodeTarget | null>(null)

export const codeTarget = target

export const openCode = (next: CodeTarget): void => {
  setTarget(next)
}

export const closeCode = (): boolean => {
  if (!target()) return false
  setTarget(null)
  return true
}
