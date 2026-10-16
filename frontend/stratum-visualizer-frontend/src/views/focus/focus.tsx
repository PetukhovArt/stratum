import { Component, For, Match, onCleanup, onMount, Show, Switch } from 'solid-js'
import { t } from '../../i18n'
import { Model } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { ViewDef, ViewProps } from '../view-def'
import { BandsScene } from './bands-scene'
import { Hops } from './columns'
import { ColumnsScene } from './columns-scene'
import { FocusDetails } from './details'
import './focus.css'
import { resolveFocus } from './focus-target'
import { FocusOutline } from './outline'
import { TerritoryStrip } from './territory-strip'

const HOPS: readonly Hops[] = [1, 2]
const HOPS_KEY = 'h'

const isTypingTarget = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement &&
  (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)

const onHopsKey = (e: KeyboardEvent): void => {
  const hasModifier = e.metaKey || e.ctrlKey || e.altKey
  if (e.key !== HOPS_KEY || hasModifier || isTypingTarget(e.target)) return
  set({ hops: nav().hops === 1 ? 2 : 1 })
}

const Toolbar: Component<{ model: Model; focus: number }> = (p) => (
  <div class="fo-toolbar">
    <Show when={nav().focusVariant === 'columns'}>
      <div class="seg" title={t('focus.hopsKey')}>
        <span class="fo-seg-lbl">{t('focus.hops')}</span>
        <For each={HOPS}>
          {(h) => (
            <button class={nav().hops === h ? 'on' : ''} data-testid={`focus-hops-${h}`} onClick={() => set({ hops: h })}>
              {h}
            </button>
          )}
        </For>
      </div>
    </Show>
    <Show when={nav().isTerritoryOpen}>
      <TerritoryStrip model={p.model} id={p.focus} />
    </Show>
    <div class="seg">
      <button
        class={nav().isTerritoryOpen ? 'on' : ''}
        data-testid="focus-territory-toggle"
        onClick={() => set({ isTerritoryOpen: !nav().isTerritoryOpen })}
      >
        {t('focus.territory')}
      </button>
    </div>
  </div>
)

const Stage: Component<ViewProps> = (props) => {
  const focus = () => resolveFocus(props.model, nav().focus)
  onMount(() => {
    window.addEventListener('keydown', onHopsKey)
    onCleanup(() => window.removeEventListener('keydown', onHopsKey))
  })
  return (
    <Show when={focus() !== null}>
      <div class="fo-stage" data-testid="focus-stage">
        <Switch>
          <Match when={nav().focusVariant === 'columns'}>
            <ColumnsScene model={props.model} focus={focus()!} />
          </Match>
          <Match when={nav().focusVariant === 'bands'}>
            <BandsScene model={props.model} focus={focus()!} />
          </Match>
        </Switch>
        <Toolbar model={props.model} focus={focus()!} />
      </div>
    </Show>
  )
}

export const focusView: ViewDef = {
  id: 'focus',
  key: '1',
  title: 'view.focus',
  Stage,
  Outline: FocusOutline,
  Details: FocusDetails,
  variants: {
    key: 'focusVariant',
    options: [
      { id: 'columns', title: 'focus.variant.columns' },
      { id: 'bands', title: 'focus.variant.bands' },
    ],
  },
}
