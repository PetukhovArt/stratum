import { t } from '../../i18n'
import { SeverityLabel } from '../../render/encoding'
import { disambiguate } from '../../render/labels'
import { Model } from '../../render/model'
import { labelOf, rulesOf, shortPath } from './paint'
import { ChangeItem, ReviewEdge, worstBreak } from './review'

export interface ItemView {
  pill: string
  severity: SeverityLabel | null
  isFixed: boolean
  main: string
  sub: string
  edge: ReviewEdge | null
}

const PATH_PREVIEW = 4

type ImportItem = Extract<ChangeItem, { kind: 'import' }>

const importText = (from: string, item: ImportItem, to: string): string =>
  `${from} ${item.edge.tag === 'removed' ? '↛' : '→'} ${to}`

const describeItem = (m: Model, item: ChangeItem): ItemView => {
  switch (item.kind) {
    case 'import': {
      const e = item.edge
      const isFixed = e.tag === 'removed'
      const severity = isFixed ? null : worstBreak(e.breaks)
      return {
        pill: isFixed ? t('mr.item.fixed') : (severity ?? ''),
        severity,
        isFixed,
        main: importText(labelOf(m, e.from, item.from), item, labelOf(m, e.to, item.to)),
        sub: rulesOf(e.breaks),
        edge: e,
      }
    }
    case 'cycle': {
      const c = item.cycle
      const n = c.members.length
      const shown = c.path.slice(0, PATH_PREVIEW).map(shortPath)
      return {
        pill: t('mr.pill.cycle'),
        severity: null,
        isFixed: false,
        main: c.grown_from === null ? t('mr.item.cycleNew', { n }) : t('mr.item.cycleGrew', { from: c.grown_from, n }),
        sub: shown.join(' → ') + (c.path.length > PATH_PREVIEW ? ' …' : ''),
        edge: null,
      }
    }
    case 'stage': {
      const s = item.stage
      return {
        pill: t('mr.pill.stage'),
        severity: null,
        isFixed: false,
        main: s.container,
        sub: s.base === null ? t('mr.item.stageNew', { head: s.head }) : t('mr.item.stage', { base: s.base, head: s.head }),
        edge: null,
      }
    }
  }
}

export const describeItems = (m: Model, items: readonly ChangeItem[]): ItemView[] => {
  const views = items.map((it) => describeItem(m, it))
  const imports = items.flatMap((it, i) => (it.kind === 'import' ? [{ it, view: views[i] }] : []))
  const side = (end: 'from' | 'to') =>
    disambiguate(imports.map(({ it, view }) => ({ key: view.main, label: labelOf(m, it.edge[end], it[end]), path: it[end] })))
  const froms = side('from')
  const tos = side('to')
  imports.forEach(({ it, view }, i) => {
    view.main = importText(froms[i], it, tos[i])
  })
  return views
}
