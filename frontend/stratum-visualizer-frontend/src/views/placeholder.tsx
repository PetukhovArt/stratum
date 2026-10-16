import { Component } from 'solid-js'
import { MessageKey, t } from '../i18n'
import { ViewId } from '../shell/nav'
import { ViewDef } from './view-def'

const EmptyPanel: Component = () => <div class="panel" />

export const placeholderView = (id: ViewId, key: string, title: MessageKey): ViewDef => ({
  id,
  key,
  title,
  Stage: () => (
    <div class="placeholder" data-testid={`placeholder-${id}`}>
      <h3>{t(title)}</h3>
      <p>{t('view.placeholder')}</p>
    </div>
  ),
  Outline: EmptyPanel,
  Details: EmptyPanel,
})
