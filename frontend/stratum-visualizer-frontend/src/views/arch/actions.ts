import { closeCode, CodeTarget, openCode } from '../../contract/code'
import { SystemContract } from '../../lib/api'
import { Model } from '../../render/model'
import { go, nav, Selection } from '../../shell/nav'
import { Box, containerBoxes } from './boxes'
import { ArchGroup, ArchPath, deepestShownPrefix, downOf, groupLevel, SelfSystem, upOf } from './path'
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

const selectionAfter = (m: Model, from: ArchPath, to: ArchPath): Selection | null => {
  if (to.level === 'code') return { kind: 'module', id: to.module }
  if (to.level === 'components' && from.level === 'code') return { kind: 'module', id: from.module }
  if (to.level === 'containers' && from.level === 'group') return { kind: 'box', key: from.group }
  if (to.level === 'containers' && (from.level === 'components' || from.level === 'code')) {
    const key = from.group?.key ?? containerBoxes(m)?.boxOfUnit.get(from.container)
    return key ? { kind: 'box', key } : null
  }
  if (to.level === 'group' && (from.level === 'components' || from.level === 'code')) {
    return { kind: 'box', key: `unit:${from.container}` }
  }
  const self = selfSystem()
  if (to.level === 'system' && from.level !== 'system' && self) return { kind: 'system', id: self.id }
  return null
}

export const goToLevel = (m: Model, to: ArchPath): void => {
  const from = archPathOf(m)
  go({ arch: to, sel: selectionAfter(m, from, to) })
  if (to.level === 'code') {
    openCode(moduleCode(m, to.module))
  } else if (from.level === 'code') {
    closeCode()
  }
}

/** A box of one unit opens that unit's components; a box of several opens a level of its units. */
export const openBox = (m: Model, box: Box, parent?: ArchGroup): void => {
  const group = { key: box.key, name: box.name }
  if (box.units.length === 1) {
    const to: ArchPath = { level: 'components', container: box.units[0] }
    goToLevel(m, parent ? { ...to, group: parent } : to)
  } else {
    goToLevel(m, groupLevel(group))
  }
}

const boxSelected = (m: Model, path: ArchPath): boolean => {
  const sel = nav().sel
  if (sel?.kind !== 'box' || (path.level !== 'containers' && path.level !== 'group')) return false
  if (path.level === 'group') {
    const unit = Number(sel.key.replace('unit:', ''))
    goToLevel(m, { level: 'components', container: unit, group: { key: path.group, name: path.name } })
    return true
  }
  const box = containerBoxes(m)?.boxes.find((b) => b.key === sel.key)
  if (!box) return false
  openBox(m, box)
  return true
}

export const goDown = (m: Model): boolean => {
  if (boxSelected(m, archPathOf(m))) return true
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
