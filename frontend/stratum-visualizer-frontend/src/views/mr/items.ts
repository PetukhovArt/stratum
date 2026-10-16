import { t } from '../../i18n'
import { SeverityLabel } from '../../render/encoding'
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

export const describeItem = (m: Model, item: ChangeItem): ItemView => {
  switch (item.kind) {
    case 'import': {
      const e = item.edge
      const isFixed = e.tag === 'removed'
      const severity = isFixed ? null : worstBreak(e.breaks)
      const arrow = isFixed ? '↛' : '→'
      return {
        pill: isFixed ? t('mr.item.fixed') : (severity ?? ''),
        severity,
        isFixed,
        main: `${labelOf(m, e.from, item.from)} ${arrow} ${labelOf(m, e.to, item.to)}`,
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
