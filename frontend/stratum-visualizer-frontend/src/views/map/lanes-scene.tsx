import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { BudgetNote } from '../../canvas/budget-note'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { CANVAS_DEFS, EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR } from '../../render/encoding'
import { edgeKey, Model, worst } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { trunc } from '../../render/labels'
import { entryOf, layerName, layerTone } from '../focus/paint'
import { Curve, defaultCurves, GRID, Lane, LANES, layoutLanes, UnitBox } from './layout'
import { MapLegend } from './legend'
import { selectedUnit, selectedUnitPair, toggleExpanded } from './selection'
import { mapDataOf, MapData } from './units'

const EDGE_WIDTH = { base: 0.8, max: 4, perLog: 0.8, highlight: 1.2 } as const
const EDGE_OPACITY = { focused: 0.9, broken: 0.85, base: 0.18, range: 0.5, perImport: 1 / 60 } as const
const NAME_CHAR_PX = 6.4
const NAME_PAD = 30
const FAN_MIN_HEIGHT = 44
const ERROR_DOT_R = 3.5
const SELECTED_STROKE = 'oklch(94% 0.005 260)'
const LABEL_FILL = 'oklch(90% 0.005 260)'
const LABEL_HALO = 'oklch(16% 0.012 260)'
const ENTRY_FILL = 'oklch(85% 0.01 260)'
const MODULE_FILL = 'oklch(42% 0.02 260)'
const META_FILL = 'oklch(62% 0.012 260)'
const DASHED_KINDS: ReadonlySet<string> = new Set(['sub_unit', 'nested'])

const unitPath = (m: Model, id: number): string => m.containers.get(id)?.path ?? ''

const EdgeMark: Component<{
  curve: Curve
  model: Model
  isFocused: () => boolean
  isHighlighted: () => boolean
}> = (props) => {
  const link = props.curve.link
  const isEmphasised = () => props.isFocused() || props.isHighlighted()
  const sev = () => (isEmphasised() ? link.worst : link.breaks > 0 ? link.breakWorst : null)
  const opacity = () => {
    if (isEmphasised()) return EDGE_OPACITY.focused
    if (link.breaks > 0) return EDGE_OPACITY.broken
    return EDGE_OPACITY.base + Math.min(EDGE_OPACITY.range, link.count * EDGE_OPACITY.perImport)
  }
  const width = () =>
    EDGE_WIDTH.base +
    Math.min(EDGE_WIDTH.max, Math.log2(link.count + 1) * EDGE_WIDTH.perLog) +
    (props.isHighlighted() ? EDGE_WIDTH.highlight : 0)
  const title = () => {
    const breaks = link.breaks ? ` · ${t('map.breaks', { n: link.breaks })}` : ''
    const rules = link.rules.length ? `\n${link.rules.join(', ')}` : ''
    return `${unitPath(props.model, link.from)}\n→ ${unitPath(props.model, link.to)}\n${t('map.imports', { n: link.count })}${breaks}${rules}`
  }
  return (
    <>
      <path
        class="map-edge"
        d={props.curve.d}
        data-edge={`unit:${link.key}`}
        data-ch={sev() ? 'sev' : 'edge'}
        data-rule={link.rules.length ? link.rules.join(' ') : undefined}
        stroke={sev() ? SEV_COLOR[sev()!] : EDGE_NEUTRAL}
        stroke-opacity={opacity()}
        stroke-width={width()}
        marker-end={CANVAS_DEFS.arrow}
        onClick={() => set({ sel: { kind: 'cell', level: 'unit', from: link.from, to: link.to } })}
      >
        <title>{title()}</title>
      </path>
      <Show when={isEmphasised()}>
        <text
          class="map-count"
          x={props.curve.label.x}
          y={props.curve.label.y}
          fill={LABEL_FILL}
          stroke={LABEL_HALO}
        >
          {link.count}
        </text>
      </Show>
    </>
  )
}

const LaneMark: Component<{ lane: Lane; model: Model }> = (props) => {
  const layer = () => props.model.layers.find((l) => l.id === props.lane.layer)
  const depends = () => {
    const names = (layer()?.depends_on ?? []).map((id) => layerName(props.model, id))
    return t('map.dependsOn', { layers: names.length ? names.join(', ') : '—' })
  }
  return (
    <g>
      <rect
        width={LANES.width}
        y={props.lane.y}
        height={props.lane.h}
        rx="10"
        fill={layerTone(props.model, props.lane.layer, LAYER_TONE.bandOff)}
        stroke={layerTone(props.model, props.lane.layer, LAYER_TONE.tintEdge)}
      />
      <rect width="4" y={props.lane.y} height={props.lane.h} rx="2" fill={layerTone(props.model, props.lane.layer)} />
      <text
        class="map-lane-name"
        x={LANES.pad}
        y={props.lane.y + 17}
        fill={layerTone(props.model, props.lane.layer, LAYER_TONE.label)}
      >
        {layerName(props.model, props.lane.layer).toUpperCase()}
      </text>
      <text class="map-lane-meta" x={LANES.width - LANES.pad} y={props.lane.y + 17} fill={META_FILL}>
        {t('map.modules', { n: props.lane.modules })} · {depends()}
      </text>
    </g>
  )
}

const UnitMark: Component<{
  box: UnitBox
  model: Model
  data: MapData
  isHovered: () => boolean
  isPaired: () => boolean
  onHover: (id: number | null) => void
}> = (props) => {
  const b = props.box
  const u = b.unit
  const info = props.data.units.get(u.id)!
  const isSelected = () => selectedUnit() === u.id
  const isRinged = () => isSelected() || props.isPaired()
  const isExpanded = b.cells !== null
  const stage = props.model.containers.get(u.id)?.stage ?? 0
  const meta = () => {
    const fan = b.h > FAN_MIN_HEIGHT || isExpanded ? ` · ${t('map.unit.fan', { in: info.fanIn, out: info.fanOut })}` : ''
    return `${u.modules.length} · s${stage}${fan}`
  }
  const title = () =>
    `${u.path}\n${t('map.unit.title', { kind: u.kind, stage, modules: t('map.modules', { n: u.modules.length }) })}\n${
      isExpanded ? t('map.unit.collapse') : t('map.unit.expand')
    }`
  return (
    <g
      class="map-unit"
      data-testid="map-unit"
      data-id={u.id}
      transform={`translate(${b.x},${b.y})`}
      onMouseEnter={() => props.onHover(u.id)}
      onMouseLeave={() => props.onHover(null)}
      onClick={() => set({ sel: { kind: 'container', id: u.id } })}
      onDblClick={() => toggleExpanded(u.id)}
    >
      <rect
        width={b.w}
        height={b.h}
        rx="6"
        fill={layerTone(props.model, u.layer, LAYER_TONE.band)}
        stroke={
          isRinged()
            ? SELECTED_STROKE
            : layerTone(props.model, u.layer, props.isHovered() ? LAYER_TONE.tintEdgeStrong : LAYER_TONE.tintEdge)
        }
        stroke-width={isRinged() ? 1.8 : 1}
        stroke-dasharray={DASHED_KINDS.has(u.kind) ? '4 2' : undefined}
      />
      <text class="map-unit-name" x="8" y="16">
        {trunc(u.name, Math.floor((b.w - NAME_PAD) / NAME_CHAR_PX))}
        <tspan class="map-unit-meta" x="8" dy="13">
          {meta()}
        </tspan>
      </text>
      <Show when={info.errors > 0}>
        <circle data-ch="sev" cx={b.w - 10} cy="12" r={ERROR_DOT_R} fill={SEV_COLOR.error}>
          <title>{t('map.errors', { n: info.errors })}</title>
        </circle>
      </Show>
      <Show when={b.cells}>
        {(cells) => (
          <For each={cells()}>
            {(cell) => {
              const sev = worst(props.model.findingsOf(cell.id))
              return (
                <rect
                  class="map-cell"
                  data-testid="map-cell"
                  data-id={cell.id}
                  data-ch={sev ? 'sev' : 'mod'}
                  x={cell.x}
                  y={cell.y}
                  width={GRID.size}
                  height={GRID.size}
                  rx="2"
                  fill={entryOf(props.model, cell.id).length ? ENTRY_FILL : MODULE_FILL}
                  stroke={sev ? SEV_COLOR[sev] : 'none'}
                  stroke-width="1.4"
                  onClick={(e) => {
                    e.stopPropagation()
                    go({ view: 'focus', focus: cell.id })
                  }}
                >
                  <title>{props.model.modules.get(cell.id)?.path}</title>
                </rect>
              )
            }}
          </For>
        )}
      </Show>
      <title>{title()}</title>
    </g>
  )
}

export const LanesScene: Component<{ model: Model }> = (props) => {
  const data = createMemo(() => mapDataOf(props.model))
  const expandedIds = createMemo(() => nav().mapExpanded)
  const expanded = createMemo(() => new Set(expandedIds()))
  const layout = createMemo(() => layoutLanes(props.model, data(), expanded()))
  const [hover, setHover] = createSignal<number | null>(null)
  const focusUnit = () => hover() ?? selectedUnit()
  const pairKey = () => {
    const p = selectedUnitPair()
    return p ? edgeKey(p.from, p.to) : null
  }
  const byDefault = createMemo(() => defaultCurves(layout(), data(), nav().mapGate, expanded(), pairKey()))
  const visible = createMemo(() => {
    const f = focusUnit()
    if (f !== null) return layout().curves.filter((c) => c.link.from === f || c.link.to === f)
    return byDefault().curves
  })
  const isPaired = (id: number) => {
    const p = selectedUnitPair()
    return p !== null && (p.from === id || p.to === id)
  }
  const fitKey = () => `lanes:${nav().mapExpanded.join(',')}`

  return (
    <Canvas
      bounds={layout().bounds}
      fitKey={fitKey()}
      legend={<MapLegend variant="lanes" />}
      overlay={
        <Show when={focusUnit() === null && byDefault().hidden > 0}>
          <BudgetNote text={t('map.budget.hidden', { n: byDefault().hidden })} />
        </Show>
      }
    >
      <g class="map" data-variant="lanes">
        <For each={layout().lanes}>{(lane) => <LaneMark lane={lane} model={props.model} />}</For>
        <g class="map-edges" fill="none">
          <For each={visible()}>
            {(curve) => (
              <EdgeMark
                curve={curve}
                model={props.model}
                isFocused={() => focusUnit() !== null}
                isHighlighted={() => curve.link.key === pairKey()}
              />
            )}
          </For>
        </g>
        <For each={[...layout().boxes.values()]}>
          {(box) => (
            <UnitMark
              box={box}
              model={props.model}
              data={data()}
              isHovered={() => hover() === box.unit.id}
              isPaired={() => isPaired(box.unit.id)}
              onHover={setHover}
            />
          )}
        </For>
      </g>
    </Canvas>
  )
}
