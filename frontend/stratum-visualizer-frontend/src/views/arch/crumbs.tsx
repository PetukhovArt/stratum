import { Component, For, Show } from 'solid-js'
import { MessageKey, t } from '../../i18n'
import { Model } from '../../render/model'
import { goToLevel } from './actions'
import { ArchLevel, ArchPath, crumbsOf } from './path'
import { systemIndex } from './systems'

const LEVEL_TITLE: Readonly<Record<ArchLevel, MessageKey>> = {
  system: 'arch.level.system',
  containers: 'arch.level.containers',
  group: 'arch.level.group',
  components: 'arch.level.components',
  code: 'arch.level.code',
}

const C4_STEPS = ['system', 'containers', 'components', 'code'] as const
type C4Step = (typeof C4_STEPS)[number]

const C4_OF: Readonly<Record<ArchLevel, C4Step>> = {
  system: 'system',
  containers: 'containers',
  group: 'containers',
  components: 'components',
  code: 'code',
}

const C4_NAME: Readonly<Record<C4Step, MessageKey>> = {
  system: 'arch.c4.system',
  containers: 'arch.c4.containers',
  components: 'arch.c4.components',
  code: 'arch.c4.code',
}

const C4_ABOUT: Readonly<Record<C4Step, MessageKey>> = {
  system: 'arch.c4.system.about',
  containers: 'arch.c4.containers.about',
  components: 'arch.c4.components.about',
  code: 'arch.c4.code.about',
}

const C4Ladder: Component<{ level: ArchLevel }> = (props) => {
  const current = () => C4_OF[props.level]
  const index = () => C4_STEPS.indexOf(current())
  return (
    <div class="arch-c4" data-testid="arch-c4" data-step={current()}>
      <span class="arch-c4-tag" aria-hidden="true">
        C4
      </span>
      <ol aria-label={t('arch.c4', { n: index() + 1, name: t(C4_NAME[current()]) })}>
        <For each={C4_STEPS}>
        {(step, i) => (
          <li
            class="arch-c4-step"
            classList={{ on: step === current(), deeper: i() > index() }}
            aria-current={step === current() ? 'step' : undefined}
            title={t(C4_ABOUT[step])}
          >
            <span class="arch-c4-n">{i() + 1}</span>
            {t(C4_NAME[step])}
          </li>
        )}
        </For>
      </ol>
    </div>
  )
}

export const C4Caption: Component<{ level: ArchLevel }> = (props) => {
  const step = () => C4_OF[props.level]
  return (
    <p class="arch-c4-caption" data-testid="arch-c4-caption">
      <strong>{t(C4_NAME[step()])}</strong> {t(C4_ABOUT[step()])}
    </p>
  )
}

export const Crumbs: Component<{ model: Model; path: ArchPath }> = (props) => {
  const crumbs = () => crumbsOf(props.path, props.model, systemIndex()?.self ?? null)
  return (
    <nav class="arch-crumbs" aria-label={t('arch.crumbs')}>
      <C4Ladder level={props.path.level} />
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
