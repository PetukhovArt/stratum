import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { t } from '../../i18n'
import { go, nav } from '../../shell/nav'
import { ViewProps } from '../view-def'
import { mostImported, resolveFocus } from './focus-target'
import { layerTone } from './paint'

const MAX_ROWS = 60
const TOP_ROWS = 25

export const FocusOutline: Component<ViewProps> = (props) => {
  const [query, setQuery] = createSignal('')
  const focus = () => resolveFocus(props.model, nav().focus)
  const matches = createMemo(() => {
    const q = query().trim().toLowerCase()
    if (!q) return null
    return [...props.model.modules.values()].filter((mod) => mod.path.toLowerCase().includes(q)).map((mod) => mod.id)
  })
  const rows = () => matches()?.slice(0, MAX_ROWS) ?? mostImported(props.model).slice(0, TOP_ROWS)
  return (
    <aside class="panel">
      <header class="panel-hd">
        <span class="panel-title">{t('focus.outline.title')}</span>
        <Show when={matches()}>
          {(list) => <span class="panel-count">{t('focus.outline.matches', { n: list().length })}</span>}
        </Show>
      </header>
      <input
        class="fo-filter"
        placeholder={t('focus.outline.filter')}
        value={query()}
        onInput={(e) => setQuery(e.currentTarget.value)}
      />
      <div class="fo-ol">
        <Show when={!matches()}>
          <div class="fo-ol-sec">{t('focus.outline.mostImported')}</div>
        </Show>
        <For each={rows()}>
          {(id) => {
            const mod = props.model.modules.get(id)!
            return (
              <div
                class={`fo-ol-mod${focus() === id ? ' on' : ''}`}
                data-testid="focus-outline-row"
                title={mod.path}
                onClick={() => go({ view: 'focus', focus: id })}
              >
                <span class="fo-sw" style={{ background: layerTone(props.model, mod.layer) }} />
                <span class="fo-nm">{props.model.label(id)}</span>
                <span class="fo-n">
                  {props.model.inn(id).length}←·→{props.model.out(id).length}
                </span>
              </div>
            )
          }}
        </For>
      </div>
    </aside>
  )
}
