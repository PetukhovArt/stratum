import { Accessor, batch, Component, createMemo, createSignal, For, Index, onCleanup, Show } from 'solid-js'
import { Dynamic } from 'solid-js/web'
import { BudgetNote } from '../../canvas/budget-note'
import { Canvas, useCanvas } from '../../canvas/canvas'
import { sharedAttr, sharedNote } from '../../features/shared-mark'
import { MessageKey, t } from '../../i18n'
import { CANVAS_DEFS, EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR, SHARED_COLOR } from '../../render/encoding'
import { edgeKey, Model, worst } from '../../render/model'
import { go, MapGate, set } from '../../shell/nav'
import { trunc } from '../../render/labels'
import { entryOf, layerName, layerTone } from '../focus/paint'
import {
  BLOCK,
  BlocksLayout,
  curveBetween,
  defaultLinks,
  GRID,
  LayerBlock,
  layoutBlocks,
  moveUnits,
  Point,
  Rect,
  Size,
  UNIT_NAME,
  UnitBox,
} from './layout'
import { MapLegend } from './legend'
import { savedPositions, SavedPositions } from './positions'
import { selectedUnit, selectedUnitPair } from './selection'
import { mapDataOf, MapData, UnitLink } from './units'

const EDGE_WIDTH = { base: 0.8, max: 4, perLog: 0.8, highlight: 1.2 } as const
const EDGE_OPACITY = { focused: 0.9, broken: 0.85, base: 0.18, range: 0.5, perImport: 1 / 60 } as const
const FAN_MIN_HEIGHT = 44
const META_CHAR_PX = 5.5
const ERROR_DOT_R = 3.5
const DRAG_THRESHOLD_PX = 3
const SELECTED_STROKE = 'oklch(94% 0.005 260)'
const LABEL_FILL = 'oklch(90% 0.005 260)'
const LABEL_HALO = 'oklch(16% 0.012 260)'
const ENTRY_FILL = 'oklch(85% 0.01 260)'
const MODULE_FILL = 'oklch(42% 0.02 260)'
const DASHED_KINDS: ReadonlySet<string> = new Set(['sub_unit', 'nested'])

const unitPath = (m: Model, id: number): string => m.containers.get(id)?.path ?? ''

export interface LanesAside {
  size: Size
  Marks: Component<{ layout: BlocksLayout; slot: Rect }>
}

export interface LanesMarks {
  unit: string
  bundle?: string
  dblClickHint?: MessageKey
}

interface Drag {
  ids: readonly number[]
  dx: number
  dy: number
}

type StartDrag = (e: PointerEvent, ids: readonly number[]) => void

const EdgeMark: Component<{
  link: UnitLink
  layout: Accessor<BlocksLayout>
  model: Model
  testId: string | undefined
  isFocused: () => boolean
  isHighlighted: () => boolean
}> = (props) => {
  const link = props.link
  const curve = createMemo(() => {
    const boxes = props.layout().boxes
    const a = boxes.get(link.from)
    const b = boxes.get(link.to)
    return a && b ? curveBetween(a, b) : null
  })
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
    <Show when={curve()}>
      {(c) => (
        <>
          <path
            class="map-edge"
            data-testid={props.testId}
            data-from={link.from}
            data-to={link.to}
            d={c().d}
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
            <text class="map-count" x={c().label.x} y={c().label.y} fill={LABEL_FILL} stroke={LABEL_HALO}>
              {link.count}
            </text>
          </Show>
        </>
      )}
    </Show>
  )
}

const BlockMark: Component<{ block: Accessor<LayerBlock>; model: Model; onDragStart: StartDrag }> = (props) => {
  const layer = () => props.block().layer
  const title = () => {
    const names = (props.model.layers.find((l) => l.id === layer())?.depends_on ?? []).map((id) =>
      layerName(props.model, id),
    )
    const depends = t('map.dependsOn', { layers: names.length ? names.join(', ') : '—' })
    return `${layerName(props.model, layer())}\n${t('map.modules', { n: props.block().modules })} · ${depends}\n${t('map.layer.dragHint')}`
  }
  return (
    <g class="map-block" data-testid="map-layer-block" data-layer={layer()}>
      <rect
        x={props.block().x}
        y={props.block().y}
        width={props.block().w}
        height={props.block().h}
        rx="8"
        fill={layerTone(props.model, layer(), LAYER_TONE.bandOff)}
        stroke={layerTone(props.model, layer(), LAYER_TONE.tintEdge)}
        pointer-events="none"
      />
      <rect
        class="map-block-handle"
        data-testid="map-layer-handle"
        data-layer={layer()}
        x={props.block().x}
        y={props.block().y}
        width={props.block().w}
        height={BLOCK.header}
        rx="8"
        fill="transparent"
        onPointerDown={(e) => props.onDragStart(e, props.block().units)}
      >
        <title>{title()}</title>
      </rect>
      <text
        class="map-lane-name"
        x={props.block().x + BLOCK.pad}
        y={props.block().y + 13}
        fill={layerTone(props.model, layer(), LAYER_TONE.label)}
        stroke={LABEL_HALO}
        pointer-events="none"
      >
        {layerName(props.model, layer()).toUpperCase()}
      </text>
    </g>
  )
}

const UnitMark: Component<{
  box: Accessor<UnitBox>
  model: Model
  data: MapData
  isHovered: () => boolean
  isPaired: () => boolean
  onHover: (id: number | null) => void
  onDblClick: (id: number) => void
  onDragStart: StartDrag
  wasDragged: () => boolean
  marks: LanesMarks
}> = (props) => {
  const u = props.box().unit
  const info = props.data.units.get(u.id)!
  const isSelected = () => selectedUnit() === u.id
  const isRinged = () => isSelected() || props.isPaired()
  const isExpanded = () => props.box().cells !== null
  const stage = props.model.containers.get(u.id)?.stage ?? 0
  const meta = () => {
    const short = `${u.modules.length} · s${stage}`
    const full = `${short} · ${t('map.unit.fan', { in: info.fanIn, out: info.fanOut })}`
    const fits = full.length * META_CHAR_PX + UNIT_NAME.pad / 2 <= props.box().w
    return fits && (props.box().h > FAN_MIN_HEIGHT || isExpanded()) ? full : short
  }
  const title = () => {
    const open = props.marks.dblClickHint
      ? t(props.marks.dblClickHint)
      : isExpanded()
        ? t('map.unit.collapse')
        : t('map.unit.expand')
    const kind = t('map.unit.title', { kind: u.kind, stage, modules: t('map.modules', { n: u.modules.length }) })
    return `${u.path}\n${kind}\n${open} · ${t('map.unit.dragHint')}`
  }
  return (
    <g
      class="map-unit"
      data-testid={props.marks.unit}
      data-id={u.id}
      transform={`translate(${props.box().x},${props.box().y})`}
      onMouseEnter={() => props.onHover(u.id)}
      onMouseLeave={() => props.onHover(null)}
      onPointerDown={(e) => props.onDragStart(e, [u.id])}
      onClick={() => {
        if (!props.wasDragged()) set({ sel: { kind: 'container', id: u.id } })
      }}
      onDblClick={() => props.onDblClick(u.id)}
    >
      <rect
        width={props.box().w}
        height={props.box().h}
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
        {trunc(u.name, Math.floor((props.box().w - UNIT_NAME.pad) / UNIT_NAME.charPx))}
        <tspan class="map-unit-meta" x="8" dy="13">
          {meta()}
        </tspan>
      </text>
      <Show when={info.errors > 0}>
        <circle data-ch="sev" cx={props.box().w - 10} cy="12" r={ERROR_DOT_R} fill={SEV_COLOR.error}>
          <title>{t('map.errors', { n: info.errors })}</title>
        </circle>
      </Show>
      <Show when={props.box().cells}>
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
                  data-shared={sharedAttr(props.model, cell.id)}
                  x={cell.x}
                  y={cell.y}
                  width={GRID.size}
                  height={GRID.size}
                  rx="2"
                  fill={entryOf(props.model, cell.id).length ? ENTRY_FILL : MODULE_FILL}
                  stroke={sev ? SEV_COLOR[sev] : props.model.isSharedByChosenFeatures(cell.id) ? SHARED_COLOR : 'none'}
                  stroke-width="1.4"
                  onClick={(e) => {
                    e.stopPropagation()
                    if (!props.wasDragged()) go({ view: 'focus', focus: cell.id })
                  }}
                >
                  <title>{(props.model.modules.get(cell.id)?.path ?? '') + sharedNote(props.model, cell.id)}</title>
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

const ResetLayout: Component<{ saved: SavedPositions }> = (props) => {
  const canvas = useCanvas()
  return (
    <button
      class="map-reset"
      data-testid="map-reset-layout"
      disabled={props.saved.at().size === 0}
      title={t('map.layout.resetTitle')}
      onClick={() => {
        props.saved.reset()
        canvas.fit()
      }}
    >
      {t('map.layout.reset')}
    </button>
  )
}

const movedAt = (
  base: BlocksLayout,
  saved: ReadonlyMap<string, Point>,
  drag: Drag | null,
): ReadonlyMap<number, Point> => {
  const at = new Map<number, Point>()
  for (const [id, box] of base.boxes) {
    const p = saved.get(box.unit.path)
    if (p) at.set(id, p)
  }
  for (const id of drag?.ids ?? []) {
    const from = at.get(id) ?? base.boxes.get(id)
    if (from && drag) at.set(id, { x: from.x + drag.dx, y: from.y + drag.dy })
  }
  return at
}

const BlocksMarks: Component<{
  model: Model
  data: MapData
  base: Accessor<BlocksLayout>
  layout: Accessor<BlocksLayout>
  visible: Accessor<readonly UnitLink[]>
  hover: Accessor<number | null>
  setHover: (id: number | null) => void
  focusUnit: () => number | null
  pairKey: () => string | null
  setDrag: (drag: Drag | null) => void
  saved: SavedPositions
  onUnitDblClick: (id: number) => void
  marks: LanesMarks
  aside?: LanesAside
}> = (props) => {
  const canvas = useCanvas()
  let wasDragged = false
  let stopDrag: (() => void) | null = null
  const startDrag: StartDrag = (e, ids) => {
    if (e.button !== 0) return
    e.stopPropagation()
    wasDragged = false
    const from = { x: e.clientX, y: e.clientY }
    const zoom = canvas.viewport().zoom
    let isMoving = false
    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - from.x
      const dy = ev.clientY - from.y
      if (!isMoving && Math.abs(dx) + Math.abs(dy) <= DRAG_THRESHOLD_PX) return
      if (!isMoving) canvas.setViewport(canvas.viewport())
      isMoving = true
      props.setDrag({ ids, dx: dx / zoom, dy: dy / zoom })
    }
    const stop = () => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', end)
      window.removeEventListener('pointercancel', end)
      stopDrag = null
    }
    const end = () => {
      stop()
      if (!isMoving) return
      wasDragged = true
      const boxes = props.layout().boxes
      const moved = new Map<string, Point>()
      for (const id of ids) {
        const box = boxes.get(id)
        if (box) moved.set(box.unit.path, { x: box.x, y: box.y })
      }
      batch(() => {
        props.saved.save(moved)
        props.setDrag(null)
      })
    }
    stopDrag?.()
    stopDrag = stop
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', end)
    window.addEventListener('pointercancel', end)
  }
  onCleanup(() => stopDrag?.())
  const isPaired = (id: number) => {
    const p = selectedUnitPair()
    return p !== null && (p.from === id || p.to === id)
  }
  const units = createMemo(() => [...props.base().boxes.values()].map((b) => b.unit))
  return (
    <g class="map" data-variant="lanes">
      <g class="map-edges" fill="none">
        <For each={props.visible()}>
          {(link) => (
            <EdgeMark
              link={link}
              layout={props.layout}
              model={props.model}
              testId={props.marks.bundle}
              isFocused={() => props.focusUnit() !== null}
              isHighlighted={() => link.key === props.pairKey()}
            />
          )}
        </For>
      </g>
      <Index each={props.layout().blocks}>
        {(block) => <BlockMark block={block} model={props.model} onDragStart={startDrag} />}
      </Index>
      <Show when={props.aside}>
        {(aside) => (
          <Show when={props.layout().reserved}>
            {(slot) => <Dynamic component={aside().Marks} layout={props.layout()} slot={slot()} />}
          </Show>
        )}
      </Show>
      <For each={units()}>
        {(unit) => (
          <Show when={props.layout().boxes.get(unit.id)}>
            {(box) => (
              <UnitMark
                box={box}
                model={props.model}
                data={props.data}
                isHovered={() => props.hover() === unit.id}
                isPaired={() => isPaired(unit.id)}
                onHover={props.setHover}
                onDblClick={props.onUnitDblClick}
                onDragStart={startDrag}
                wasDragged={() => wasDragged}
                marks={props.marks}
              />
            )}
          </Show>
        )}
      </For>
    </g>
  )
}

export const LanesScene: Component<{
  model: Model
  view: 'map' | 'arch'
  gate: MapGate
  expanded: readonly number[]
  onUnitDblClick: (id: number) => void
  marks: LanesMarks
  aside?: LanesAside
}> = (props) => {
  const data = createMemo(() => mapDataOf(props.model))
  const expandedIds = createMemo(() => props.expanded)
  const expanded = createMemo(() => new Set(expandedIds()))
  const base = createMemo(() => layoutBlocks(props.model, expanded(), props.aside?.size ?? null))
  const saved = savedPositions(() => props.model.project, props.view)
  const [drag, setDrag] = createSignal<Drag | null>(null)
  const layout = createMemo(() => moveUnits(base(), movedAt(base(), saved.at(), drag())))
  const [hover, setHover] = createSignal<number | null>(null)
  const focusUnit = () => hover() ?? selectedUnit()
  const pairKey = () => {
    const p = selectedUnitPair()
    return p ? edgeKey(p.from, p.to) : null
  }
  const byDefault = createMemo(() => defaultLinks(base(), data(), props.gate, expanded(), pairKey()))
  const visible = createMemo(() => {
    const f = focusUnit()
    if (f === null) return byDefault().links
    const boxes = base().boxes
    return data().links.filter((l) => (l.from === f || l.to === f) && boxes.has(l.from) && boxes.has(l.to))
  })
  const fitKey = () => `blocks:${expandedIds().join(',')}`

  return (
    <Canvas
      bounds={layout().bounds}
      fitKey={fitKey()}
      legend={<MapLegend variant="lanes" />}
      overlay={
        <>
          <ResetLayout saved={saved} />
          <Show when={focusUnit() === null && byDefault().hidden > 0}>
            <BudgetNote text={t('map.budget.hidden', { n: byDefault().hidden })} />
          </Show>
        </>
      }
    >
      <BlocksMarks
        model={props.model}
        data={data()}
        base={base}
        layout={layout}
        visible={visible}
        hover={hover}
        setHover={setHover}
        focusUnit={focusUnit}
        pairKey={pairKey}
        setDrag={setDrag}
        saved={saved}
        onUnitDblClick={props.onUnitDblClick}
        marks={props.marks}
        aside={props.aside}
      />
    </Canvas>
  )
}
