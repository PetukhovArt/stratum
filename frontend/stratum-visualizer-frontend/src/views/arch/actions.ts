import { closeCode, CodeTarget, openCode } from '../../contract/code'
import { SystemContract } from '../../lib/api'
import { Model } from '../../render/model'
import { go, nav, Selection } from '../../shell/nav'
import { ArchPath, deepestShownPrefix, downOf, SelfSystem, upOf } from './path'
import { systemIndex } from './systems'

const selfSystem = (): SelfSystem => systemIndex()?.self ?? null

export const archPathOf = (m: Model): ArchPath => deepestShownPrefix(nav().arch, m, selfSystem())

export const moduleCode = (m: Model, id: number): CodeTarget => ({
  name: m.label(id),
  signature: null,
  decl: { path: m.modules.get(id)?.path ?? '', line: 1 },
  site: null,
})

export const contractCode = (m: Model, contract: SystemContract, caller = contract.callers[0]): CodeTarget => {
  const path = caller ? m.modules.get(caller.module)?.path : undefined
  return {
    name: contract.id,
    signature: contract.signature,
    decl: contract.decl,
    site: path ? { kind: 'call', path, line: caller.line } : null,
  }
}

const selectionAfter = (from: ArchPath, to: ArchPath): Selection | null => {
  if (to.level === 'code') return { kind: 'module', id: to.module }
  if (to.level === 'components' && from.level === 'code') return { kind: 'module', id: from.module }
  if (to.level === 'containers' && (from.level === 'components' || from.level === 'code')) {
    return { kind: 'container', id: from.container }
  }
  const self = selfSystem()
  if (to.level === 'system' && from.level !== 'system' && self) return { kind: 'system', id: self.id }
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
  const to = downOf(archPathOf(m), nav().sel, selfSystem())
  if (!to) return false
  goToLevel(m, to)
  return true
}

export const goUp = (m: Model): boolean => {
  const to = upOf(archPathOf(m), selfSystem())
  if (!to) return false
  goToLevel(m, to)
  return true
}
