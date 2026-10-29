import type { MessagesOf } from '../i18n/types'

export const en = {
  'features.label': 'features',
  'features.title': 'Show only the modules of the chosen features',
  'features.all': 'all',
  'features.clear': 'Show all features',
  'features.hideCommon': 'hide common >',
  'features.hideCommonTitle': 'Hide the modules that more than {k} features use',
  'features.commonLimit': 'Most features a shown module may belong to',
  'features.shared': 'shared',
  'features.sharedTitle': 'shared by two or more chosen features',
} as const

export const ru: MessagesOf<typeof en> = {
  'features.label': 'фичи',
  'features.title': 'Показывать только модули выбранных фич',
  'features.all': 'все',
  'features.clear': 'Показать все фичи',
  'features.hideCommon': 'скрыть общие >',
  'features.hideCommonTitle': 'Скрыть модули, которые используют больше {k} фич',
  'features.commonLimit': 'Сколько фич может использовать показанный модуль',
  'features.shared': 'общий',
  'features.sharedTitle': 'используется двумя и более выбранными фичами',
}
