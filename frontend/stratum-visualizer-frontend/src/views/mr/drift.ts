import { createSignal } from 'solid-js'
import { fetchDrift } from '../../lib/api'
import { ContainerKind, EdgeKind } from '../../types'
import { SeverityLabel } from '../../render/encoding'

export type Change = 'added' | 'removed'

export interface EdgeChange {
  /** Root-relative paths: a removed edge may name a module that exists only at the base. */
  from: string
  to: string
  kind: EdgeKind
  change: Change
  breaks: [slug: string, severity: SeverityLabel][]
}

export interface CycleChange {
  members: string[]
  grown_from: number | null
  /** A head cycle through at least one added edge, first path repeated last; empty if none. */
  path: string[]
}

export interface StageChange {
  container: string
  kind: ContainerKind
  base: number | null
  head: number
}

export interface DriftReport {
  base: { ref: string; sha: string }
  markdown: string
  /** Indices into `edges` of the imports that break rules, in the markdown's order. */
  breaking: number[]
  edges: EdgeChange[]
  cycles: CycleChange[]
  stages: StageChange[]
}

type DriftState = { kind: 'loading' } | { kind: 'absent' } | { kind: 'ready'; drift: DriftReport }

const [state, setState] = createSignal<DriftState>({ kind: 'loading' })
let isRequested = false

const driftState = async (): Promise<DriftState> => {
  try {
    return { kind: 'ready', drift: (await fetchDrift()) as DriftReport }
  } catch {
    return { kind: 'absent' }
  }
}

/** Fetches `/api/drift` on the first read; `null` until it answers, and for good without `--base`. */
export const driftReport = (): DriftReport | null => {
  if (!isRequested) {
    isRequested = true
    void driftState().then(setState)
  }
  const s = state()
  return s.kind === 'ready' ? s.drift : null
}
