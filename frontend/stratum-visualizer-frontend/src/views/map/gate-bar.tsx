import { Component, For } from 'solid-js'
import { t } from '../../i18n'
import { MAP_GATES, MapGate, nav, set } from '../../shell/nav'
import { TOP_IMPORTS } from './units'

export const GateBar: Component<{ total: number }> = (props) => {
  const title = (gate: MapGate) => {
    if (gate === 'rules') return t('map.gate.rules')
    if (gate === 'top') return t('map.gate.top', { n: TOP_IMPORTS })
    return t('map.gate.all', { n: props.total })
  }
  return (
    <div class="map-toolbar">
      <div class="seg">
        <span class="map-seg-lbl">{t('map.edges')}</span>
        <For each={MAP_GATES}>
          {(gate) => (
            <button
              class={nav().mapGate === gate ? 'on' : ''}
              data-testid={`map-gate-${gate}`}
              onClick={() => set({ mapGate: gate })}
            >
              {title(gate)}
            </button>
          )}
        </For>
      </div>
    </div>
  )
}
