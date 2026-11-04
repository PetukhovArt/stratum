import type { SystemInfo } from '../../lib/api'
import type { Model } from '../../render/model'
import type { Selection } from '../../shell/nav'

export type ArchPath =
  | { level: 'system' }
  | { level: 'containers' }
  | { level: 'group'; group: string; name: string }
  | { level: 'components'; container: number; group?: ArchGroup }
  | { level: 'code'; container: number; module: number; group?: ArchGroup }

export interface ArchGroup {
  key: string
  name: string
}

export type ArchLevel = ArchPath['level']

export const ARCH_TOP: ArchPath = { level: 'system' }
export const ARCH_CONTAINERS: ArchPath = { level: 'containers' }

export type SelfSystem = SystemInfo | null

const isId = (value: unknown): value is number => Number.isInteger(value) && (value as number) >= 0

export const parseArchPath = (raw: unknown): ArchPath => {
  if (typeof raw !== 'object' || raw === null || !('level' in raw)) return ARCH_TOP
  const container = 'container' in raw ? raw.container : undefined
  const module = 'module' in raw ? raw.module : undefined
  const group = 'group' in raw ? parseGroup(raw.group) : undefined
  if (raw.level === 'containers') return ARCH_CONTAINERS
  if (raw.level === 'group' && 'group' in raw && typeof raw.group === 'string' && 'name' in raw && typeof raw.name === 'string') {
    return { level: 'group', group: raw.group, name: raw.name }
  }
  if (raw.level === 'components' && isId(container)) return withGroup({ level: 'components', container }, group)
  if (raw.level === 'code' && isId(container) && isId(module)) return withGroup({ level: 'code', container, module }, group)
  return ARCH_TOP
}

const parseGroup = (raw: unknown): ArchGroup | undefined =>
  typeof raw === 'object' && raw !== null && 'key' in raw && typeof raw.key === 'string' && 'name' in raw && typeof raw.name === 'string'
    ? { key: raw.key, name: raw.name }
    : undefined

const withGroup = <P extends ArchPath>(path: P, group: ArchGroup | undefined): P => (group ? { ...path, group } : path)

export const groupLevel = (group: ArchGroup): ArchPath => ({ level: 'group', group: group.key, name: group.name })

const hasUnit = (m: Model, id: number): boolean => m.units.some((u) => u.id === id)

export const deepestShownPrefix = (path: ArchPath, m: Model, self: SelfSystem): ArchPath => {
  if (path.level === 'system') return self ? path : ARCH_CONTAINERS
  if (path.level === 'containers' || path.level === 'group') return path
  if (!hasUnit(m, path.container)) return path.group ? groupLevel(path.group) : ARCH_CONTAINERS
  const components: ArchPath = withGroup({ level: 'components', container: path.container }, path.group)
  if (path.level === 'components') return components
  return m.modules.has(path.module) && m.unitOf(path.module) === path.container ? path : components
}

export const upOf = (path: ArchPath, self: SelfSystem): ArchPath | null => {
  switch (path.level) {
    case 'system':
      return null
    case 'containers':
      return self ? ARCH_TOP : null
    case 'group':
      return ARCH_CONTAINERS
    case 'components':
      return path.group ? groupLevel(path.group) : ARCH_CONTAINERS
    case 'code':
      return withGroup({ level: 'components', container: path.container }, path.group)
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
    case 'group':
      return sel?.kind === 'container' ? { level: 'components', container: sel.id } : null
    case 'components':
    case 'code':
      return sel?.kind === 'module' ? withGroup({ level: 'code', container: path.container, module: sel.id }, path.group) : null
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
  if (path.level === 'group') return [...top, { level: 'group', label: path.name, path }]
  const group: Crumb[] = path.group ? [{ level: 'group', label: path.group.name, path: groupLevel(path.group) }] : []
  const container: Crumb = {
    level: 'components',
    label: m.unitLabel(path.container),
    path: withGroup({ level: 'components', container: path.container }, path.group),
  }
  if (path.level === 'components') return [...top, ...group, container]
  return [...top, ...group, container, { level: 'code', label: m.label(path.module), path }]
}
