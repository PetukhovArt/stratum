import { Component, createMemo, For, Show } from 'solid-js'
import { t } from '../../i18n'
import { MAP_GATES, MapGate, nav, set } from '../../shell/nav'
import { ViewDef, ViewProps } from '../view-def'
import { LanesScene } from './lanes-scene'
import { LayersScene } from './layers-scene'
import './map.css'
import { MapDetails, MapOutline } from './panels'
import { mapDataOf, TOP_IMPORTS } from './units'

const GateBar: Component<{ total: number }> = (props) => {
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

const Stage: Component<ViewProps> = (props) => {
  const data = createMemo(() => mapDataOf(props.model))
  return (
    <div class="map-stage" data-testid="map-stage" data-variant={nav().mapVariant}>
      <Show when={nav().mapVariant === 'lanes'} fallback={<LayersScene model={props.model} />}>
        <LanesScene model={props.model} />
        <GateBar total={data().links.length} />
      </Show>
    </div>
  )
}

export const mapView: ViewDef = {
  id: 'map',
  key: '4',
  title: 'view.map',
  Stage,
  Outline: MapOutline,
  Details: MapDetails,
  variants: {
    key: 'mapVariant',
    options: [
      { id: 'lanes', title: 'map.variant.lanes' },
      { id: 'layers', title: 'map.variant.layers' },
    ],
  },
}
