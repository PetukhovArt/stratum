import { t } from '../i18n'
import type { Model } from '../render/model'

export const sharedAttr = (m: Model, id: number): 'true' | undefined =>
  m.isSharedByChosenFeatures(id) ? 'true' : undefined

export const sharedNote = (m: Model, id: number): string =>
  m.isSharedByChosenFeatures(id) ? `\n${t('features.sharedTitle')}` : ''
