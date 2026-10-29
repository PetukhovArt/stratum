import type { SystemInfo } from '../../lib/api'
import type { Model } from '../../render/model'
import type { Selection } from '../../shell/nav'

export type ArchPath =
  | { level: 'system' }
  | { level: 'containers' }
  | { level: 'components'; container: number }
  | { level: 'code'; container: number; module: number }

export type ArchLevel = ArchPath['level']

export const ARCH_TOP: ArchPath = { level: 'system' }
export const ARCH_CONTAINERS: ArchPath = { level: 'containers' }

/** The self system when the server declares systems; `null` leaves Architecture without the System level. */
export type SelfSystem = SystemInfo | null

const isId = (value: unknown): value is number => Number.isInteger(value) && (value as number) >= 0

export const parseArchPath = (raw: unknown): ArchPath => {
  if (typeof raw !== 'object' || raw === null || !('level' in raw)) return ARCH_TOP
  const container = 'container' in raw ? raw.container : undefined
  const module = 'module' in raw ? raw.module : undefined
  if (raw.level === 'containers') return ARCH_CONTAINERS
  if (raw.level === 'components' && isId(container)) return { level: 'components', container }
  if (raw.level === 'code' && isId(container) && isId(module)) return { level: 'code', container, module }
  return ARCH_TOP
}

const hasUnit = (m: Model, id: number): boolean => m.units.some((u) => u.id === id)

export const deepestShownPrefix = (path: ArchPath, m: Model, self: SelfSystem): ArchPath => {
  if (path.level === 'system') return self ? path : ARCH_CONTAINERS
  if (path.level === 'containers' || !hasUnit(m, path.container)) return ARCH_CONTAINERS
  const components: ArchPath = { level: 'components', container: path.container }
  if (path.level === 'components') return components
  return m.modules.has(path.module) && m.unitOf(path.module) === path.container ? path : components
}

export const upOf = (path: ArchPath, self: SelfSystem): ArchPath | null => {
  switch (path.level) {
    case 'system':
      return null
    case 'containers':
      return self ? ARCH_TOP : null
    case 'components':
      return ARCH_CONTAINERS
    case 'code':
      return { level: 'components', container: path.container }
    default: {
      const never: never = path
      return never
    }
  }
}

export const downOf = (path: ArchPath, sel: Selection | null, self: SelfSystem): ArchPath | null => {
  switch (path.level) {
    case 'system':
      return sel?.kind === 'system' && sel.id === self?.id ? ARCH_CONTAINERS : null
    case 'containers':
      return sel?.kind === 'container' ? { level: 'components', container: sel.id } : null
    case 'components':
    case 'code':
      return sel?.kind === 'module' ? { level: 'code', container: path.container, module: sel.id } : null
    default: {
      const never: never = path
      return never
    }
  }
}

export interface Crumb {
  level: ArchLevel
  label: string
  path: ArchPath
}

export const crumbsOf = (path: ArchPath, m: Model, self: SelfSystem): Crumb[] => {
  const system: Crumb[] = self ? [{ level: 'system', label: self.name, path: ARCH_TOP }] : []
  if (path.level === 'system') return system
  const top: Crumb[] = [...system, { level: 'containers', label: m.project, path: ARCH_CONTAINERS }]
  if (path.level === 'containers') return top
  const container: Crumb = {
    level: 'components',
    label: m.unitLabel(path.container),
    path: { level: 'components', container: path.container },
  }
  if (path.level === 'components') return [...top, container]
  return [...top, container, { level: 'code', label: m.label(path.module), path }]
}
