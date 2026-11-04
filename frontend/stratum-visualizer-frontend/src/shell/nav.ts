import { createSignal } from 'solid-js'
import { DEFAULT_COMMON_LIMIT } from '../features/scope'
import { ARCH_TOP, ArchPath, parseArchPath } from '../views/arch/path'

export const VIEW_IDS = ['focus', 'dsm', 'mr', 'set', 'arch', 'bench'] as const
export type ViewId = (typeof VIEW_IDS)[number]

export type Selection =
  | { kind: 'module'; id: number }
  | { kind: 'container'; id: number }
  | { kind: 'edge'; from: number; to: number }
  | { kind: 'cell'; level: 'layer' | 'unit'; from: number; to: number }
  | { kind: 'imports'; pairs: readonly (readonly [from: number, to: number])[] }
  | { kind: 'system'; id: string }
  | { kind: 'channel'; to: string }
  | { kind: 'port'; box: string; system: string }
  | { kind: 'box'; key: string }
  | { kind: 'link'; from: string; to: string }

export interface NavState {
  view: ViewId
  focus: number | null
  hops: 1 | 2
  isTerritoryOpen: boolean
  focusVariant: 'columns' | 'bands'
  dsmLevel: 'layer' | 'unit'
  mrVariant: 'region' | 'card'
  setVariant: 'ring' | 'arc' | 'matrix'
  set: readonly number[]
  arch: ArchPath
  features: readonly string[] | null
  isHidingCommon: boolean
  commonLimit: number
  sel: Selection | null
}

export type NavPatch = Partial<NavState>

const STORAGE_KEY = 'stratum.nav'
const HISTORY_CAP = 40

const DEFAULT_STATE: NavState = {
  view: 'focus',
  focus: null,
  hops: 1,
  isTerritoryOpen: true,
  focusVariant: 'columns',
  dsmLevel: 'layer',
  mrVariant: 'region',
  setVariant: 'ring',
  set: [],
  arch: ARCH_TOP,
  features: null,
  isHidingCommon: false,
  commonLimit: DEFAULT_COMMON_LIMIT,
  sel: null,
}

const oneOf = <T>(value: unknown, allowed: readonly T[], fallback: T): T =>
  allowed.includes(value as T) ? (value as T) : fallback

const isCount = (value: unknown): value is number => Number.isInteger(value) && (value as number) >= 1

const restore = (): NavState => {
  let raw: Record<string, unknown> = {}
  try {
    raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}') ?? {}
  } catch {
    raw = {}
  }
  const d = DEFAULT_STATE
  return {
    view: oneOf(raw.view, VIEW_IDS, d.view),
    focus: typeof raw.focus === 'number' ? raw.focus : d.focus,
    hops: oneOf(raw.hops, [1, 2] as const, d.hops),
    isTerritoryOpen: typeof raw.isTerritoryOpen === 'boolean' ? raw.isTerritoryOpen : d.isTerritoryOpen,
    focusVariant: oneOf(raw.focusVariant, ['columns', 'bands'] as const, d.focusVariant),
    dsmLevel: oneOf(raw.dsmLevel, ['layer', 'unit'] as const, d.dsmLevel),
    mrVariant: oneOf(raw.mrVariant, ['region', 'card'] as const, d.mrVariant),
    setVariant: oneOf(raw.setVariant, ['ring', 'arc', 'matrix'] as const, d.setVariant),
    set: Array.isArray(raw.set) ? raw.set.filter((id): id is number => typeof id === 'number') : d.set,
    arch: parseArchPath(raw.arch),
    features: Array.isArray(raw.features)
      ? raw.features.filter((id): id is string => typeof id === 'string')
      : d.features,
    isHidingCommon: typeof raw.isHidingCommon === 'boolean' ? raw.isHidingCommon : d.isHidingCommon,
    commonLimit: isCount(raw.commonLimit) ? raw.commonLimit : d.commonLimit,
    sel: null,
  }
}

const [state, setState] = createSignal<NavState>(restore())
const [history, setHistory] = createSignal<readonly NavState[]>([])

export const nav = state
export const historyDepth = (): number => history().length

const commit = (next: NavState): void => {
  setState(next)
  const { sel: _sel, ...persisted } = next
  localStorage.setItem(STORAGE_KEY, JSON.stringify(persisted))
}

export const set = (patch: NavPatch): void => commit({ ...state(), ...patch })

export const go = (patch: NavPatch): void => {
  setHistory([...history(), state()].slice(-HISTORY_CAP))
  commit({ ...state(), sel: null, ...patch })
}

export const back = (): void => {
  if (state().sel) {
    set({ sel: null })
    return
  }
  const past = history()
  const previous = past[past.length - 1]
  if (!previous) return
  setHistory(past.slice(0, -1))
  commit(previous)
}
