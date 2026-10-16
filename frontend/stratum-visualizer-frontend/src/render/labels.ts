/** Advance of one JetBrains Mono glyph, in em. */
const MONO_ADVANCE = 0.6
const ELLIPSIS = '…'
const MIN_KEPT_HEAD = 3

export const monoWidth = (text: string, size: number): number => text.length * size * MONO_ADVANCE

export const monoChars = (width: number, size: number): number => Math.max(1, Math.floor(width / (size * MONO_ADVANCE)))

export const trunc = (s: string, n: number): string =>
  s.length <= n ? s : `${s.slice(0, Math.ceil(n / 2) - 1)}${ELLIPSIS}${s.slice(-Math.floor(n / 2))}`

/**
 * Shortens a `/` path to at most `max` characters by dropping whole middle segments for `…`. The file name stays
 * whole unless it alone is longer than `max`; the directories nearest to it go last.
 */
export const ellipsizePath = (path: string, max: number): string => {
  if (path.length <= max) return path
  const rest = path.split('/')
  const base = rest.pop()!
  if (!rest.length || base.length + 2 > max) return trunc(base, max)
  let tail = base
  while (rest.length > 1 && `${ELLIPSIS}/${rest[rest.length - 1]}/${tail}`.length <= max) tail = `${rest.pop()}/${tail}`
  const head = rest[0]
  if (rest.length === 1) {
    const room = max - tail.length - 2
    return room >= MIN_KEPT_HEAD ? `${head.slice(0, room)}${ELLIPSIS}/${tail}` : `${ELLIPSIS}/${tail}`
  }
  const withHead = `${head}/${ELLIPSIS}/${tail}`
  return withHead.length <= max ? withHead : `${ELLIPSIS}/${tail}`
}

export const splitPath = (path: string): { dir: string; base: string } => {
  const cut = path.lastIndexOf('/')
  return { dir: path.slice(0, Math.max(cut, 0)), base: path.slice(cut + 1) }
}

const hasSuffix =(path: string, suffix: string): boolean => path === suffix || path.endsWith(`/${suffix}`)

export interface Labelled {
  key: string
  label: string
  path: string
}

/**
 * `label` for each item no other item with the same `key` and a different `path` collides with; otherwise the
 * shortest trailing part of `path`, at least as many segments as `label`, that none of those items ends with.
 */
export const disambiguate = (items: readonly Labelled[]): string[] => {
  const byKey = new Map<string, string[]>()
  for (const it of items) byKey.set(it.key, [...(byKey.get(it.key) ?? []), it.path])
  return items.map((it) => {
    const others = byKey.get(it.key)!.filter((p) => p !== it.path)
    if (!others.length) return it.label
    const segments = it.path.split('/')
    for (let depth = it.label.split('/').length; depth < segments.length; depth++) {
      const suffix = segments.slice(-depth).join('/')
      if (!others.some((p) => hasSuffix(p, suffix))) return suffix
    }
    return it.path
  })
}
