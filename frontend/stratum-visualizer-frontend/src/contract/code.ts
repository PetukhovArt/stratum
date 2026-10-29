import { createSignal } from 'solid-js'
import { Loc } from '../lib/api'

export interface CodeTarget {
  name: string
  decl: Loc | null
  site: Loc | null
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
