import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { CANVAS_DEFS, EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR } from '../../render/encoding'
import { edgeKey, Model } from '../../render/model'
import { set } from '../../shell/nav'
import { trunc } from '../../render/labels'
import { layerName, layerTone, shortRule } from '../focus/paint'
import { LAYER_NODE, LayerCurve, LayerNode, layoutLayers } from './layout'
import { MapLegend } from './legend'
import { selectedLayerPair } from './selection'
import { mapDataOf, MapData } from './units'

const EDGE_WIDTH = { base: 0.8, max: 5, perLog: 0.6, highlight: 1.2 } as const
const EDGE_OPACITY = { finding: 0.9, undeclared: 0.75, base: 0.25, range: 0.5, perImport: 1 / 800 } as const
const UNIT_CHARS = 26
const RULE_CHARS = 34
const DIM_TEXT = 'oklch(62% 0.012 260)'
const ROW_TEXT = 'oklch(84% 0.006 260)'
const NAME_TEXT = 'oklch(94% 0.005 260)'
const SLUG_TEXT = 'oklch(80% 0.01 260)'
const LABEL_HALO = 'oklch(16% 0.012 260)'

const LayerEdge: Component<{ curve: LayerCurve; model: Model }> = (props) => {
  const link = props.curve.link
  const isHighlighted = () => {
    const p = selectedLayerPair()
    return p !== null && edgeKey(p.from, p.to) === link.key
  }
  const opacity = () => {
    if (link.worst || isHighlighted()) return EDGE_OPACITY.finding
    if (!link.isDeclared) return EDGE_OPACITY.undeclared
    return EDGE_OPACITY.base + Math.min(EDGE_OPACITY.range, link.count * EDGE_OPACITY.perImport)
  }
  const width = () =>
    EDGE_WIDTH.base +
    Math.min(EDGE_WIDTH.max, Math.log2(link.count + 1) * EDGE_WIDTH.perLog) +
    (isHighlighted() ? EDGE_WIDTH.highlight : 0)
  const slugs = link.rules.map(shortRule).join(', ')
  const title = () =>
    `${layerName(props.model, link.from)} → ${layerName(props.model, link.to)}: ${t('map.imports', { n: link.count })} ${t(
      'map.unitPairs',
      { n: link.unitPairs },
    )}\n${link.isDeclared ? t('map.declared') : t('map.undeclared')}${slugs ? `\n${slugs}` : ''}`
  return (
    <>
      <path
        class="map-edge"
        d={props.curve.d}
        data-edge={`layer:${link.key}`}
        data-ch={link.worst ? 'sev' : 'edge'}
        data-rule={link.rules.length ? link.rules.join(' ') : undefined}
        data-declared={link.isDeclared ? 'yes' : 'no'}
        stroke={link.worst ? SEV_COLOR[link.worst] : EDGE_NEUTRAL}
        stroke-opacity={opacity()}
        stroke-width={width()}
        marker-end={CANVAS_DEFS.arrow}
        onClick={() => set({ sel: { kind: 'cell', level: 'layer', from: link.from, to: link.to } })}
      >
        <title>{title()}</title>
      </path>
      <Show when={slugs}>
        <text
          class="map-slug"
          x={props.curve.label.x}
          y={props.curve.label.y}
          text-anchor={link.isDeclared ? 'end' : 'start'}
          fill={SLUG_TEXT}
          stroke={LABEL_HALO}
        >
          {trunc(slugs, RULE_CHARS)}
        </text>
      </Show>
    </>
  )
}

const LayerBox: Component<{
  node: LayerNode
  model: Model
  data: MapData
  isOpen: boolean
  onToggle: () => void
}> = (props) => {
  const L = LAYER_NODE
  const errors = () => props.node.units.reduce((n, u) => n + (props.data.units.get(u.id)?.errors ?? 0), 0)
  return (
    <g
      class="map-layer"
      data-testid="map-layer"
      data-id={props.node.layer}
      transform={`translate(0,${props.node.y})`}
      onClick={() => props.onToggle()}
    >
      <rect
        width={L.w}
        height={props.node.h}
        rx="8"
        fill={layerTone(props.model, props.node.layer, LAYER_TONE.band)}
        stroke={layerTone(props.model, props.node.layer, LAYER_TONE.tintEdgeStrong)}
      />
      <rect width="5" height={props.node.h} rx="2" fill={layerTone(props.model, props.node.layer)} />
      <text class="map-layer-name" x="16" y="22" fill={NAME_TEXT}>
        {layerName(props.model, props.node.layer)}
      </text>
      <text class="map-layer-meta" x="16" y="40" fill={DIM_TEXT}>
        {t('map.units', { n: props.node.units.length })} · {t('map.modules', { n: props.node.modules })}
      </text>
      <Show when={errors() > 0}>
        <text data-ch="sev" class="map-layer-meta" x={L.w - 12} y="22" text-anchor="end" fill={SEV_COLOR.error}>
          {t('map.errors', { n: errors() })}
        </text>
      </Show>
      <Show when={props.isOpen}>
        <For each={props.node.units.slice(0, L.maxRows)}>
          {(u, i) => (
            <g
              class="map-layer-unit"
              transform={`translate(16,${L.h + i() * L.row})`}
              onClick={(e) => {
                e.stopPropagation()
                set({ mapVariant: 'lanes', sel: { kind: 'container', id: u.id } })
              }}
            >
              <text class="map-layer-meta" y="10" fill={ROW_TEXT}>
                {trunc(u.name, UNIT_CHARS)}
              </text>
              <text class="map-layer-meta" x={L.w - 28} y="10" text-anchor="end" fill={DIM_TEXT}>
                {u.modules.length} · {t('map.unit.fan', {
                  in: props.data.units.get(u.id)?.fanIn ?? 0,
                  out: props.data.units.get(u.id)?.fanOut ?? 0,
                })}
              </text>
              <title>{u.path}</title>
            </g>
          )}
        </For>
      </Show>
    </g>
  )
}

export const LayersScene: Component<{ model: Model }> = (props) => {
  const data = createMemo(() => mapDataOf(props.model))
  const [open, setOpen] = createSignal<number | null>(null)
  const layout = createMemo(() => layoutLayers(props.model, data(), open()))
  return (
    <Canvas bounds={layout().bounds} fitKey={`layers:${open()}`} legend={<MapLegend variant="layers" />}>
      <g class="map" data-variant="layers">
        <g class="map-edges" fill="none">
          <For each={layout().curves}>{(curve) => <LayerEdge curve={curve} model={props.model} />}</For>
        </g>
        <For each={layout().nodes}>
          {(node) => (
            <LayerBox
              node={node}
              model={props.model}
              data={data()}
              isOpen={open() === node.layer}
              onToggle={() => setOpen(open() === node.layer ? null : node.layer)}
            />
          )}
        </For>
        <text class="map-layer-meta" x={LAYER_NODE.w + 16} y={-6} fill={DIM_TEXT}>
          {t('map.layers.undeclared')}
        </text>
        <text class="map-layer-meta" x={-16} y={-6} text-anchor="end" fill={DIM_TEXT}>
          {t('map.layers.declared')}
        </text>
      </g>
    </Canvas>
  )
}
