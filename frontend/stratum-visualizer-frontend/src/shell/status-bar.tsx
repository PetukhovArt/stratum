import { Component, Show } from 'solid-js'
import { t } from '../i18n'
import { Model } from '../render/model'
import { VIEWS, viewById } from '../views'
import { historyDepth, nav } from './nav'
import { isPanelCollapsed, PanelSide, togglePanel } from './panels'

export const StatusBar: Component<{ model: Model }> = (props) => {
  const firstKey = VIEWS[0].key
  const lastKey = VIEWS[VIEWS.length - 1].key
  return (
    <footer class="status">
      <DockButton side="left" />
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
      <DockButton side="right" />
    </footer>
  )
}

const DockButton: Component<{ side: PanelSide }> = (props) => {
  const isOpen = () => !isPanelCollapsed(props.side)
  const title = () => {
    const key = props.side === 'left' ? 'Ctrl [' : 'Ctrl ]'
    const action = t(isOpen() ? 'shell.panel.hide' : 'shell.panel.show')
    return `${action}: ${t(props.side === 'left' ? 'shell.panel.left' : 'shell.panel.right')} (${key})`
  }
  return (
    <button
      class="dock-btn"
      classList={{ on: isOpen() }}
      data-testid={`panel-toggle-${props.side}`}
      aria-pressed={isOpen()}
      aria-label={title()}
      title={title()}
      onClick={() => togglePanel(props.side)}
    >
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
        <rect x="1.5" y="2.5" width="13" height="11" rx="2" fill="none" stroke="currentColor" />
        <rect x={props.side === 'left' ? 2 : 10} y="3" width="4" height="10" rx="1" fill="currentColor" opacity={isOpen() ? 1 : 0.35} />
      </svg>
    </button>
  )
}
