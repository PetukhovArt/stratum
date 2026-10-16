import { Component, Show } from 'solid-js'
import { t } from '../i18n'
import { Model } from '../render/model'
import { VIEWS, viewById } from '../views'
import { historyDepth, nav } from './nav'

export const StatusBar: Component<{ model: Model }> = (props) => {
  const firstKey = VIEWS[0].key
  const lastKey = VIEWS[VIEWS.length - 1].key
  return (
    <footer class="status">
      <Show
        when={historyDepth() > 0}
        fallback={
          <span data-testid="status-graph">
            {t('shell.stats.modules', { n: props.model.modules.size })} ·{' '}
            {t('shell.stats.edges', { n: props.model.edges.length })} ·{' '}
            {t('shell.stats.violations', { n: props.model.violations.length })}
          </span>
        }
      >
        <span>
          <kbd>Esc</kbd> {t('shell.status.back', { n: historyDepth() })}
        </span>
      </Show>
      <span class="sp" />
      <span>
        <kbd>{firstKey}</kbd>–<kbd>{lastKey}</kbd> {t('shell.status.views')}
      </span>
      <Show when={viewById(nav().view).variants}>
        <span>
          <kbd>v</kbd> {t('shell.status.variant')}
        </span>
      </Show>
      <span>
        <kbd>f</kbd> {t('shell.status.fit')}
      </span>
      <span>
        <kbd>Ctrl [</kbd>
        <kbd>Ctrl ]</kbd> {t('shell.status.panels')}
      </span>
      <span>
        <kbd>/</kbd> {t('shell.status.palette')}
      </span>
    </footer>
  )
}
