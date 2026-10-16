import { Model, sccs } from '../../render/model'
import { NavPatch } from '../../shell/nav'
import { SnapshotLayer } from '../../types'
import { Subjects } from './subjects'

const CROSS_LAYER_RULE = 'stratum/no-cross-layer-import'
const STRIP = '[data-testid="territory-strip"]'
const BLOCK_STROKE = 2

export type Asker = 'developer' | 'reviewer' | 'architect' | 'agent'

export interface Step {
  label: string
  patch: NavPatch
}

export interface TaskDef {
  order: number
  id: string
  question: string
  who: Asker
  view: string
  answer: string
  steps: readonly Step[]
  isAnswerShown: () => boolean
}

export interface PendingTask {
  order: number
  id: string
  question: string
  who: Asker
  view: string
  reason: 'phase 2' | 'needs the merge request view'
}

type TaskBlock = (m: Model, s: Subjects) => TaskDef[]

const words = (text: string | null): string[] => (text ?? '').split(/[\s,]+/).filter(Boolean)

const chips = (): HTMLElement[] => [...document.querySelectorAll<HTMLElement>(`${STRIP} .fo-chip`)]

const sameList = (a: readonly string[], b: readonly string[]): boolean => {
  const sortedB = [...b].sort()
  return a.length === b.length && [...a].sort().every((x, i) => x === sortedB[i])
}

const outsideTerritory = (m: Model, id: number): SnapshotLayer[] => {
  const allowed = m.territoryOf(id).may_import_layers
  return m.layers.filter((l) => !allowed.includes(l.id))
}

const openFocus = (id: number, path: string): Step => ({
  label: `jump to ${path}`,
  patch: { view: 'focus', focus: id, hops: 1, focusVariant: 'columns', isTerritoryOpen: true },
})

const mayImportTask = (m: Model, s: Subjects): TaskDef => {
  const subject = s.list.find((x) => x.role === 'private') ?? s.list.find((x) => x.role === 'findings') ?? s.hub
  const own = m.modules.get(subject.id)!.layer
  const banned = outsideTerritory(m, subject.id)[0]
  const layer = banned ?? m.layers.find((l) => l.id === own)!
  const isLayerChip = (chip: HTMLElement) =>
    banned
      ? chip.classList.contains('no') && words(chip.textContent).includes(layer.name)
      : !chip.classList.contains('no') && chip.textContent?.trim() === layer.name
  return {
    order: 1,
    id: 'may-import',
    question: `May ${subject.path} import from ${layer.name}?`,
    who: 'developer',
    view: 'focus',
    answer: banned ? `no: ${layer.name} is outside its territory` : `yes: ${layer.name} is in its territory`,
    steps: [openFocus(subject.id, subject.path)],
    isAnswerShown: () => chips().some(isLayerChip),
  }
}

const largestImporterLayer = (m: Model, id: number): { layer: number; importers: number[] } => {
  const byLayer = new Map<number, number[]>()
  for (const from of m.inn(id)) {
    const layer = m.modules.get(from)!.layer
    byLayer.set(layer, [...(byLayer.get(layer) ?? []), from])
  }
  const [top] = [...byLayer.entries()].sort((a, b) => b[1].length - a[1].length || a[0] - b[0])
  return top ? { layer: top[0], importers: top[1] } : { layer: m.modules.get(id)!.layer, importers: [] }
}

const importersTask = (m: Model, s: Subjects): TaskDef => {
  const hub = s.hub
  const { layer, importers } = largestImporterLayer(m, hub.id)
  const layerName = m.layers.find((l) => l.id === layer)?.name ?? String(layer)
  const isListed = (column: Element, id: number) =>
    column.querySelector(`[data-testid="focus-row"][data-id="${id}"]`) !== null ||
    column.querySelector(`[data-testid="focus-group"][data-key="-1:${m.groupOf(id).key}"]`) !== null
  return {
    order: 2,
    id: 'importers-in-layer',
    question: `Which ${layerName} modules import ${hub.path}?`,
    who: 'developer',
    view: 'focus',
    answer: `${importers.length} modules in ${layerName}`,
    steps: [openFocus(hub.id, hub.path)],
    isAnswerShown: () => {
      const column = document.querySelector('[data-testid="focus-column"][data-side="-1"]')
      if (!column) return false
      if (!importers.length) return column.querySelector('[data-testid="focus-group"]') === null
      const isHeaded = (column.textContent ?? '').includes(layerName.toUpperCase())
      return isHeaded && importers.every((id) => isListed(column, id))
    },
  }
}

const mayNotTask = (m: Model, s: Subjects): TaskDef => {
  const hub = s.hub
  const rules = m.territoryOf(hub.id).may_not.map((f) => f.rule)
  const banned = outsideTerritory(m, hub.id).map((l) => l.name)
  const parts = [...(banned.length ? [`layers ${banned.join(', ')}`] : []), ...rules]
  return {
    order: 8,
    id: 'may-not-import',
    question: `What may ${hub.path} not import?`,
    who: 'agent',
    view: 'focus',
    answer: parts.join(' · ') || 'nothing',
    steps: [openFocus(hub.id, hub.path)],
    isAnswerShown: () => {
      if (!document.querySelector(STRIP)) return false
      const refused = chips().filter((c) => c.classList.contains('no'))
      const shownRules = refused.map((c) => c.dataset.rule).filter((r): r is string => r !== undefined)
      const shownLayers = refused.filter((c) => c.dataset.rule === undefined).flatMap((c) => words(c.textContent))
      return sameList(shownRules, rules) && banned.every((name) => shownLayers.includes(name))
    },
  }
}

const focusTasks: TaskBlock = (m, s) => [mayImportTask(m, s), importersTask(m, s), mayNotTask(m, s)]

const crossLayerPairs = (m: Model): string[] => {
  const layerOf = (id: number) => m.modules.get(id)?.layer
  const pairs = new Set<string>()
  for (const v of m.violations) {
    if (v.rule !== CROSS_LAYER_RULE || !v.edge) continue
    const from = layerOf(v.edge.from)
    const to = layerOf(v.edge.to)
    if (from !== undefined && to !== undefined && from !== to) pairs.add(`${from}>${to}`)
  }
  return [...pairs].sort()
}

const layerPairsTask = (m: Model): TaskDef => {
  const expected = crossLayerPairs(m)
  const name = (id: string) => m.layers.find((l) => String(l.id) === id)?.name ?? id
  return {
    order: 5,
    id: 'layer-pairs-against',
    question: 'Which layer pairs import against depends_on?',
    who: 'architect',
    view: 'dsm',
    answer: expected.map((p) => p.split('>').map(name).join('→')).join(', ') || 'none',
    steps: [{ label: 'open Matrix at layer level', patch: { view: 'dsm', dsmLevel: 'layer' } }],
    isAnswerShown: () => {
      const matrix = document.querySelector('.dsm[data-level="layer"]')
      if (!matrix) return false
      const marked = [...matrix.querySelectorAll('rect[data-ch="sev"][data-edge]')].map((el) =>
        (el.getAttribute('data-edge') ?? '').replace(/^layer:/, ''),
      )
      return sameList(marked, expected)
    },
  }
}

const unitCycleSizes = (m: Model): number[] => {
  const out = new Map<number, Set<number>>()
  for (const e of m.edges) {
    const from = m.unitOf(e.from)
    const to = m.unitOf(e.to)
    if (from !== to) out.set(from, (out.get(from) ?? new Set()).add(to))
  }
  return sccs(
    m.units.map((u) => u.id),
    (u) => out.get(u) ?? [],
  )
    .map((c) => c.length)
    .filter((n) => n > 1)
    .sort((a, b) => b - a)
}

const unitCyclesTask = (m: Model): TaskDef => {
  const sizes = unitCycleSizes(m)
  return {
    order: 7,
    id: 'unit-cycles',
    question: 'Do units form import cycles, and how big?',
    who: 'architect',
    view: 'dsm',
    answer: sizes.length ? `${sizes.length} cycle blocks, largest ${sizes[0]} units` : 'no cycles',
    steps: [{ label: 'open Matrix at unit level', patch: { view: 'dsm', dsmLevel: 'unit' } }],
    isAnswerShown: () => {
      const matrix = document.querySelector('.dsm[data-level="unit"]')
      if (!matrix) return false
      const blocks = [...matrix.querySelectorAll('rect.dsm-block')]
      if (!blocks.length) return sizes.length === 0
      const cell = Number(matrix.querySelector('.dsm-cells rect')?.getAttribute('width')) + 1
      const shown = blocks.map((b) => Math.round((Number(b.getAttribute('width')) - BLOCK_STROKE) / cell))
      return sameList(shown.map(String), sizes.map(String))
    },
  }
}

const dsmTasks: TaskBlock = (m) => [layerPairsTask(m), unitCyclesTask(m)]

const fanInHotspotTask = (m: Model): TaskDef => {
  const importers = new Map<number, number>()
  for (const e of m.unitEdges) importers.set(e.to, (importers.get(e.to) ?? 0) + 1)
  const [hot] = m.units
    .map((u) => ({ unit: u, fanIn: importers.get(u.id) ?? 0 }))
    .sort((a, b) => b.fanIn - a.fanIn || a.unit.id - b.unit.id)
  return {
    order: 6,
    id: 'fan-in-hotspot',
    question: 'Which unit is the biggest fan-in hotspot?',
    who: 'architect',
    view: 'map',
    answer: hot ? `${hot.unit.path} (${hot.fanIn} importing units)` : 'no units',
    steps: [{ label: 'open Container map', patch: { view: 'map', mapVariant: 'lanes', sel: null } }],
    isAnswerShown: () => {
      const top = document.querySelector('[data-testid="map-hotspots"] [data-id]')
      return hot ? top?.getAttribute('data-id') === String(hot.unit.id) : top === null
    },
  }
}

const mapTasks: TaskBlock = (m) => [fanInHotspotTask(m)]

export const TASK_BLOCKS: readonly TaskBlock[] = [focusTasks, dsmTasks, mapTasks]

export const PENDING_TASKS: readonly PendingTask[] = [
  {
    order: 3,
    id: 'cycle-cut',
    question: 'Which import breaks the largest module cycle?',
    who: 'reviewer',
    view: 'set',
    reason: 'phase 2',
  },
  {
    order: 4,
    id: 'mr-breaks',
    question: 'What does this merge request break?',
    who: 'reviewer',
    view: 'mr',
    reason: 'needs the merge request view',
  },
]
