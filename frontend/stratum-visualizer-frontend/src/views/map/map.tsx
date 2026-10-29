import { Component, createMemo, Show } from 'solid-js'
import { nav } from '../../shell/nav'
import { ViewDef, ViewProps } from '../view-def'
import { GateBar } from './gate-bar'
import { LanesScene } from './lanes-scene'
import { LayersScene } from './layers-scene'
import './map.css'
import { MapDetails, MapOutline } from './panels'
import { toggleExpanded } from './selection'
import { mapDataOf } from './units'

const MARKS = { unit: 'map-unit' } as const

const Stage: Component<ViewProps> = (props) => {
  const data = createMemo(() => mapDataOf(props.model))
  return (
    <div class="map-stage" data-testid="map-stage" data-variant={nav().mapVariant}>
      <Show when={nav().mapVariant === 'lanes'} fallback={<LayersScene model={props.model} />}>
        <LanesScene
          view="map"
          model={props.model}
          gate={nav().mapGate}
          expanded={nav().mapExpanded}
          onUnitDblClick={toggleExpanded}
          marks={MARKS}
        />
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
