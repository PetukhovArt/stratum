import { Accessor, createMemo, createSignal } from 'solid-js'
import { Point } from './layout'

const STORAGE_PREFIX = 'stratum.positions'

export interface SavedPositions {
  /** Moved units by unit path, top-left corner in scene units. */
  at: Accessor<ReadonlyMap<string, Point>>
  save: (moved: ReadonlyMap<string, Point>) => void
  reset: () => void
}

const isPair = (v: unknown): v is [number, number] =>
  Array.isArray(v) && v.length === 2 && v.every((n) => typeof n === 'number' && Number.isFinite(n))

const parse = (raw: string | null): Map<string, Point> => {
  const at = new Map<string, Point>()
  let json: unknown
  try {
    json = JSON.parse(raw ?? '{}')
  } catch {
    return at
  }
  if (typeof json !== 'object' || json === null || Array.isArray(json)) return at
  for (const [path, v] of Object.entries(json)) {
    if (isPair(v)) at.set(path, { x: v[0], y: v[1] })
  }
  return at
}

const round = (n: number): number => Math.round(n * 10) / 10

/** Positions the user dragged, kept in localStorage under one key per project and view. */
export const savedPositions = (project: Accessor<string>, view: string): SavedPositions => {
  const key = () => `${STORAGE_PREFIX}:${project()}:${view}`
  const [version, setVersion] = createSignal(0)
  const at = createMemo(() => {
    version()
    return parse(localStorage.getItem(key()))
  })
  const write = (next: ReadonlyMap<string, Point>) => {
    if (next.size) {
      const entries = [...next].map(([path, p]) => [path, [round(p.x), round(p.y)]])
      localStorage.setItem(key(), JSON.stringify(Object.fromEntries(entries)))
    } else {
      localStorage.removeItem(key())
    }
    setVersion((v) => v + 1)
  }
  return {
    at,
    save: (moved) => write(new Map([...at(), ...moved])),
    reset: () => write(new Map()),
  }
}
