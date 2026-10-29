import { Component, createSignal, For, onCleanup, Show } from 'solid-js'
import { t } from '../i18n'
import type { Feature } from '../lib/api'
import { go, nav } from '../shell/nav'
import './features.css'

const SUMMARY_NAMES = 2

const toggled = (chosen: readonly string[], id: string): string[] =>
  chosen.includes(id) ? chosen.filter((x) => x !== id) : [...chosen, id]

const namesOf = (features: readonly Feature[], chosen: readonly string[]): string[] =>
  features.filter((f) => chosen.includes(f.id)).map((f) => f.name)

export const FeatureFilter: Component<{ features: readonly Feature[]; chosen: readonly string[] }> = (props) => {
  const [isOpen, setOpen] = createSignal(false)
  let root: HTMLDivElement | undefined

  const onPointerDown = (e: PointerEvent) => {
    if (root && e.target instanceof Node && !root.contains(e.target)) setOpen(false)
  }
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== 'Escape' || !isOpen()) return
    e.stopImmediatePropagation()
    setOpen(false)
  }
  window.addEventListener('pointerdown', onPointerDown)
  window.addEventListener('keydown', onKeyDown, true)
  onCleanup(() => {
    window.removeEventListener('pointerdown', onPointerDown)
    window.removeEventListener('keydown', onKeyDown, true)
  })

  const onLimit = (e: Event & { currentTarget: HTMLInputElement }) => {
    const n = Number(e.currentTarget.value)
    if (Number.isInteger(n) && n >= 1) go({ commonLimit: n })
    else e.currentTarget.value = String(nav().commonLimit)
  }

  const names = () => namesOf(props.features, props.chosen)
  const hidden = () => Math.max(0, names().length - SUMMARY_NAMES)

  return (
    <div class="ff" ref={root}>
      <button
        class={`ff-btn${props.chosen.length ? ' on' : ''}`}
        data-testid="feature-filter"
        aria-haspopup="listbox"
        aria-expanded={isOpen()}
        title={names().length ? names().join(', ') : t('features.title')}
        onClick={() => setOpen(!isOpen())}
      >
        <span class="ff-lbl">{t('features.label')}</span>
        <span class="ff-sum">{names().length ? names().slice(0, SUMMARY_NAMES).join(', ') : t('features.all')}</span>
        <Show when={hidden()}>
          <span class="ff-n" data-testid="feature-more">+{hidden()}</span>
        </Show>
        <Show when={nav().isHidingCommon}>
          <span class="ff-n" data-testid="feature-hiding-common">
            {t('features.hidingCommon', { k: nav().commonLimit })}
          </span>
        </Show>
      </button>
      <Show when={isOpen()}>
        <div class="ff-pop" role="listbox" aria-multiselectable="true" aria-label={t('features.title')}>
          <For each={props.features}>
            {(f) => (
              <button
                class={`ff-chip${props.chosen.includes(f.id) ? ' on' : ''}`}
                role="option"
                aria-selected={props.chosen.includes(f.id)}
                data-testid="feature-option"
                data-id={f.id}
                title={f.id}
                onClick={() => go({ features: toggled(props.chosen, f.id) })}
              >
                <span class="ff-name">{f.name}</span>
                <span class="ff-n">{f.modules.length}</span>
              </button>
            )}
          </For>
          <Show when={props.chosen.length}>
            <button class="ff-clear" data-testid="feature-clear" onClick={() => go({ features: [] })}>
              {t('features.clear')}
            </button>
          </Show>
          <div class="seg ff-common ff-foot" title={t('features.hideCommonTitle', { k: nav().commonLimit })}>
            <button
              class={nav().isHidingCommon ? 'on' : ''}
              data-testid="feature-hide-common"
              aria-pressed={nav().isHidingCommon}
              onClick={() => go({ isHidingCommon: !nav().isHidingCommon })}
            >
              {t('features.hideCommon')}
            </button>
            <input
              class="ff-k"
              type="number"
              min="1"
              data-testid="feature-common-limit"
              aria-label={t('features.commonLimit')}
              value={nav().commonLimit}
              onChange={onLimit}
            />
          </div>
        </div>
      </Show>
    </div>
  )
}
