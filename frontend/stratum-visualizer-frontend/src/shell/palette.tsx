import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { t } from '../i18n'
import { Model } from '../render/model'
import { availableViews } from '../views'
import { go } from './nav'

const MAX_ROWS = 60

interface PaletteItem {
  key: string
  kind: string
  label: string
  sub?: string
  run: () => void
}

const itemsOf = (model: Model): PaletteItem[] => [
  ...availableViews().map((v) => ({
    key: `v-${v.id}`,
    kind: t('shell.palette.kind.view'),
    label: t(v.title),
    sub: v.key,
    run: () => go({ view: v.id }),
  })),
  ...[...model.modules.values()].map((m) => ({
    key: `m-${m.id}`,
    kind: t('shell.palette.kind.focus'),
    label: m.path,
    run: () => go({ view: 'focus', focus: m.id }),
  })),
]

export const Palette: Component<{ model: Model; onClose: () => void }> = (props) => {
  const [query, setQuery] = createSignal('')
  const [cursor, setCursor] = createSignal(0)
  const items = createMemo(() => itemsOf(props.model))
  const matches = createMemo(() => {
    const q = query().trim().toLowerCase()
    const all = q ? items().filter((it) => `${it.label} ${it.sub ?? ''}`.toLowerCase().includes(q)) : items()
    return all.slice(0, MAX_ROWS)
  })

  const run = (item: PaletteItem) => {
    props.onClose()
    item.run()
  }

  const onKeyDown = (e: KeyboardEvent) => {
    e.stopPropagation()
    const list = matches()
    if (e.key === 'Escape') {
      props.onClose()
    } else if (e.key === 'ArrowDown') {
      e.preventDefault()
      setCursor(Math.min(list.length - 1, cursor() + 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setCursor(Math.max(0, cursor() - 1))
    } else if (e.key === 'Enter' && list[cursor()]) {
      run(list[cursor()])
    }
  }

  return (
    <div class="pal-scrim" onClick={props.onClose}>
      <div class="pal" role="dialog" onClick={(e) => e.stopPropagation()}>
        <input
          ref={(el) => queueMicrotask(() => el.focus())}
          placeholder={t('shell.palette.placeholder')}
          value={query()}
          onInput={(e) => {
            setQuery(e.currentTarget.value)
            setCursor(0)
          }}
          onKeyDown={onKeyDown}
        />
        <div class="pal-list">
          <For each={matches()}>
            {(item, i) => (
              <div
                class={`pal-row ${i() === cursor() ? 'on' : ''}`}
                onMouseEnter={() => setCursor(i())}
                onClick={() => run(item)}
              >
                <span class="k">{item.kind}</span>
                <span class="nm">{item.label}</span>
                <Show when={item.sub}>
                  <kbd>{item.sub}</kbd>
                </Show>
              </div>
            )}
          </For>
          <Show when={matches().length === 0}>
            <div class="pal-row">
              <span class="nm">{t('shell.palette.noMatch')}</span>
            </div>
          </Show>
        </div>
        <div class="pal-foot">
          <span>
            <kbd>↑↓</kbd> {t('shell.palette.move')}
          </span>
          <span>
            <kbd>↵</kbd> {t('shell.palette.run')}
          </span>
          <span>
            <kbd>Esc</kbd> {t('shell.palette.close')}
          </span>
        </div>
      </div>
    </div>
  )
}
