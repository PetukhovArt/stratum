export const normalizePath = (p: string): string => p.replace(/\\/g, '/')

const escapeRe = (s: string): string => s.replace(/[.+^${}()|[\]\\]/g, '\\$&')

const patternToRegex = (pattern: string): RegExp => {
  if (!/[*?/]/.test(pattern)) {
    return new RegExp(`.*${escapeRe(pattern)}.*`, 'i')
  }
  const body = escapeRe(pattern)
    .replace(/\?/g, '.')
    .replace(/\*\*\//g, '§§DSTAR_SLASH§§')
    .replace(/\*\*/g, '§§DSTAR§§')
    .replace(/\*/g, '[^/]*')
    .replace(/§§DSTAR_SLASH§§/g, '(?:.+/)?')
    .replace(/§§DSTAR§§/g, '.*')
  return new RegExp(`^(?:.+/)?${body}$`, 'i')
}

export const globToRegex = (glob: string): ((p: string) => boolean) | null => {
  if (!glob) return null
  const parts = glob
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
  if (parts.length === 0) return null
  const regexes = parts.map((p) => {
    const isNegated = p.startsWith('!')
    return { isNegated, re: patternToRegex(normalizePath(isNegated ? p.slice(1) : p)) }
  })
  const hasPositive = regexes.some((r) => !r.isNegated)
  return (path: string) => {
    const p = normalizePath(path)
    let isIncluded = !hasPositive
    for (const { isNegated, re } of regexes) {
      if (re.test(p)) {
        isIncluded = !isNegated
      }
    }
    return isIncluded
  }
}
