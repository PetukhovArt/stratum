import type { Model } from '../../render/model'
import type { Selection } from '../../shell/nav'

export type ArchPath =
  | { level: 'containers' }
  | { level: 'components'; container: number }
  | { level: 'code'; container: number; module: number }

export type ArchLevel = ArchPath['level']

export const ARCH_TOP: ArchPath = { level: 'containers' }

const isId = (value: unknown): value is number => Number.isInteger(value) && (value as number) >= 0

export const parseArchPath = (raw: unknown): ArchPath => {
  if (typeof raw !== 'object' || raw === null || !('level' in raw)) return ARCH_TOP
  const container = 'container' in raw ? raw.container : undefined
  const module = 'module' in raw ? raw.module : undefined
  if (raw.level === 'components' && isId(container)) return { level: 'components', container }
  if (raw.level === 'code' && isId(container) && isId(module)) return { level: 'code', container, module }
  return ARCH_TOP
}

const hasUnit = (m: Model, id: number): boolean => m.units.some((u) => u.id === id)

export const deepestShownPrefix = (path: ArchPath, m: Model): ArchPath => {
  if (path.level === 'containers' || !hasUnit(m, path.container)) return ARCH_TOP
  const components: ArchPath = { level: 'components', container: path.container }
  if (path.level === 'components') return components
  return m.modules.has(path.module) && m.unitOf(path.module) === path.container ? path : components
}

export const upOf = (path: ArchPath): ArchPath | null => {
  switch (path.level) {
    case 'containers':
      return null
    case 'components':
      return ARCH_TOP
    case 'code':
      return { level: 'components', container: path.container }
    default: {
      const never: never = path
      return never
    }
  }
}

export const downOf = (path: ArchPath, sel: Selection | null): ArchPath | null => {
  if (path.level === 'containers' && sel?.kind === 'container') return { level: 'components', container: sel.id }
  if (path.level !== 'containers' && sel?.kind === 'module') {
    return { level: 'code', container: path.container, module: sel.id }
  }
  return null
}

export interface Crumb {
  level: ArchLevel
  label: string
  path: ArchPath
}

export const crumbsOf = (path: ArchPath, m: Model): Crumb[] => {
  const top: Crumb = { level: 'containers', label: m.project, path: ARCH_TOP }
  if (path.level === 'containers') return [top]
  const container: Crumb = {
    level: 'components',
    label: m.unitLabel(path.container),
    path: { level: 'components', container: path.container },
  }
  if (path.level === 'components') return [top, container]
  return [top, container, { level: 'code', label: m.label(path.module), path }]
}
