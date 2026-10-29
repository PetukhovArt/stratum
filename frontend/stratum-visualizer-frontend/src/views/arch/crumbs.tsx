import { Component, For, Show } from 'solid-js'
import { MessageKey, t } from '../../i18n'
import { Model } from '../../render/model'
import { goToLevel } from './actions'
import { ArchLevel, ArchPath, crumbsOf } from './path'
import { systemIndex } from './systems'

const LEVEL_TITLE: Readonly<Record<ArchLevel, MessageKey>> = {
  system: 'arch.level.system',
  containers: 'arch.level.containers',
  components: 'arch.level.components',
  code: 'arch.level.code',
}

export const Crumbs: Component<{ model: Model; path: ArchPath }> = (props) => {
  const crumbs = () => crumbsOf(props.path, props.model, systemIndex()?.self ?? null)
  return (
    <nav class="arch-crumbs" aria-label={t('arch.crumbs')}>
      <For each={crumbs()}>
        {(crumb, i) => {
          const isCurrent = () => i() === crumbs().length - 1
          return (
            <>
              <Show when={i() > 0}>
                <span class="arch-crumb-sep">›</span>
              </Show>
              <button
                class={`arch-crumb${isCurrent() ? ' on' : ''}`}
                data-testid="arch-crumb"
                data-level={crumb.level}
                aria-current={isCurrent() ? 'location' : undefined}
                title={t(LEVEL_TITLE[crumb.level])}
                onClick={() => !isCurrent() && goToLevel(props.model, crumb.path)}
              >
                <span class="arch-crumb-level">{t(LEVEL_TITLE[crumb.level])}</span>
                <span class="arch-crumb-name">{crumb.label}</span>
              </button>
            </>
          )
        }}
      </For>
      <span class="sp" />
      <span class="arch-keys">
        <kbd>Enter</kbd> {t('arch.keys.down')} <kbd>Backspace</kbd> {t('arch.keys.up')}
      </span>
    </nav>
  )
}
