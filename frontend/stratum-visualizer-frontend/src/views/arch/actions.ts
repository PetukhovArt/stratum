import { closeCode, CodeTarget, openCode } from '../../contract/code'
import { Model } from '../../render/model'
import { go, nav, Selection } from '../../shell/nav'
import { ArchPath, deepestShownPrefix, downOf, upOf } from './path'

export const archPathOf = (m: Model): ArchPath => deepestShownPrefix(nav().arch, m)

export const moduleCode = (m: Model, id: number): CodeTarget => ({
  name: m.label(id),
  decl: { path: m.modules.get(id)?.path ?? '', line: 1 },
  site: null,
})

const selectionAfter = (from: ArchPath, to: ArchPath): Selection | null => {
  if (to.level === 'code') return { kind: 'module', id: to.module }
  if (to.level === 'components' && from.level === 'code') return { kind: 'module', id: from.module }
  if (to.level === 'containers' && from.level !== 'containers') return { kind: 'container', id: from.container }
  return null
}

export const goToLevel = (m: Model, to: ArchPath): void => {
  const from = archPathOf(m)
  go({ arch: to, sel: selectionAfter(from, to) })
  if (to.level === 'code') {
    openCode(moduleCode(m, to.module))
  } else if (from.level === 'code') {
    closeCode()
  }
}

export const goDown = (m: Model): boolean => {
  const to = downOf(archPathOf(m), nav().sel)
  if (!to) return false
  goToLevel(m, to)
  return true
}

export const goUp = (m: Model): boolean => {
  const to = upOf(archPathOf(m))
  if (!to) return false
  goToLevel(m, to)
  return true
}
