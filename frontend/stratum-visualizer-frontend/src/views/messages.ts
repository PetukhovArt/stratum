import type { MessagesOf } from '../i18n/types'

export const en = {
  'view.focus': 'Module focus',
  'view.dsm': 'Matrix',
  'view.mr': 'Merge request',
  'view.map': 'Container map',
} as const

export const ru: MessagesOf<typeof en> = {
  'view.focus': 'Фокус на модуле',
  'view.dsm': 'Матрица',
  'view.mr': 'Merge request',
  'view.map': 'Карта контейнеров',
}
