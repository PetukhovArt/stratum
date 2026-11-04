import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { t } from '../../i18n'
import { Model, worst } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { PathText } from '../../shell/path-text'
import { layerTone, shortRule } from '../focus/paint'
import { ViewProps } from '../view-def'
import { unitDataOf, UnitLink } from './units'

const PAIR_LIST = 40

interface Pair {
  from: number
  to: number
}

export const selectedUnit = (): number | null => {
  const sel = nav().sel
  return sel?.kind === 'container' ? sel.id : null
}

export const selectedUnitPair = (): Pair | null => {
  const sel = nav().sel
  return sel?.kind === 'cell' && sel.level === 'unit' ? { from: sel.from, to: sel.to } : null
}

const unitPath = (m: Model, id: number): string => m.containers.get(id)?.path ?? ''

export const Swatch: Component<{ model: Model; layer: number }> = (props) => (
  <span class="arch-sw" style={{ background: layerTone(props.model, props.layer) }} />
)

export const LinkDetails: Component<{ model: Model; link: UnitLink }> = (props) => {
  const pairs = createMemo(() =>
    props.model.edges
      .filter((e) => props.model.unitOf(e.from) === props.link.from && props.model.unitOf(e.to) === props.link.to)
      .map((e) => ({ ...e, findings: props.model.findingsOnEdge(e.from, e.to) }))
      .sort((a, b) => b.findings.length - a.findings.length || a.from - b.from || a.to - b.to),
  )
  return (
    <div class="det-section" data-testid="map-link">
      <h4>
        {props.model.unitLabel(props.link.from)} → {props.model.unitLabel(props.link.to)}
      </h4>
      <div class="det-path">
        <PathText path={unitPath(props.model, props.link.from)} />
        <br />→ <PathText path={unitPath(props.model, props.link.to)} />
      </div>
      <p class="arch-note">
        {t('arch.imports', { n: props.link.count })}
        <Show when={props.link.breaks}>
          {' · '}
          <span class="c-err" data-ch="sev">
            {t('arch.breaks', { n: props.link.breaks })}
          </span>
        </Show>
      </p>
      <Show when={props.link.rules.length}>
        <div class="arch-rules mono">{props.link.rules.map(shortRule).join(', ')}</div>
      </Show>
      <h4>{t('arch.pairs')}</h4>
      <div class="arch-rows">
        <For each={pairs().slice(0, PAIR_LIST)}>
          {(e) => {
            const sev = worst(e.findings)
            return (
              <button
                class="arch-row"
                title={`${props.model.modules.get(e.from)?.path}\n→ ${props.model.modules.get(e.to)?.path}`}
                onClick={() => go({ view: 'focus', focus: e.from })}
              >
                <span class="nm mono">
                  {props.model.label(e.from)} → {props.model.label(e.to)}
                </span>
                <Show when={sev}>{(s) => <span class={`arch-dot sev-${s()}`} data-ch="sev" />}</Show>
              </button>
            )
          }}
        </For>
        <Show when={pairs().length > PAIR_LIST}>
          <div class="empty">{t('arch.more', { n: pairs().length - PAIR_LIST })}</div>
        </Show>
      </div>
    </div>
  )
}

export const UnitsOutline: Component<ViewProps> = (props) => {
  const [query, setQuery] = createSignal('')
  const data = createMemo(() => unitDataOf(props.model))
  const groups = createMemo(() => {
    const q = query().trim().toLowerCase()
    return props.model.layers
      .map((layer) => ({
        layer,
        units: props.model.units
          .filter((u) => u.layer === layer.id && (!q || u.path.toLowerCase().includes(q)))
          .sort((a, b) => b.modules.length - a.modules.length || a.id - b.id),
      }))
      .filter((g) => g.units.length)
  })
  return (
    <aside class="panel">
      <header class="panel-hd">
        <span class="panel-title">{t('arch.outline')}</span>
        <span class="panel-count">{props.model.units.length}</span>
      </header>
      <input
        class="arch-filter"
        placeholder={t('arch.filter')}
        value={query()}
        onInput={(e) => setQuery(e.currentTarget.value)}
      />
      <div class="outline-tree">
        <For each={groups()}>
          {(g) => (
            <>
              <div class="arch-ol-layer">
                <Swatch model={props.model} layer={g.layer.id} />
                {g.layer.name}
              </div>
              <For each={g.units}>
                {(u) => (
                  <div
                    class={`arch-ol-unit${selectedUnit() === u.id ? ' on' : ''}`}
                    data-testid="map-outline-row"
                    title={u.path}
                    onClick={() => set({ sel: { kind: 'container', id: u.id } })}
                  >
                    <span class="nm">{u.name}</span>
                    <Show when={(data().units.get(u.id)?.errors ?? 0) > 0}>
                      <span class="arch-dot sev-error" data-ch="sev" />
                    </Show>
                    <span class="n">{u.modules.length}</span>
                  </div>
                )}
              </For>
            </>
          )}
        </For>
      </div>
    </aside>
  )
}
