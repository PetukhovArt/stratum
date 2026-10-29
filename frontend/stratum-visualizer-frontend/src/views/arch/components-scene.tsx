import { Component, createMemo, For, Show } from 'solid-js'
import { BudgetNote } from '../../canvas/budget-note'
import { Canvas } from '../../canvas/canvas'
import { sharedAttr, sharedNote } from '../../features/shared-mark'
import { t } from '../../i18n'
import { trunc } from '../../render/labels'
import { CANVAS_DEFS, EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR, SHARED_COLOR } from '../../render/encoding'
import { Model, Unit, worst } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { layerTone } from '../focus/paint'
import { goToLevel } from './actions'
import { ImportArrow, layoutComponents, marksWithinBudget, ModuleNode, NODE, ROOT_SEGMENT, SegmentBox } from './components'

const LABEL_CHAR_PX = 6
const LABEL_PAD = 12
const SELECTED_STROKE = 'oklch(94% 0.005 260)'
const NODE_FILL = 'oklch(30% 0.02 260)'
const LABEL_FILL = 'oklch(90% 0.005 260)'
const LABEL_HALO = 'oklch(16% 0.012 260)'
const ARROW_OPACITY = { focused: 0.9, base: 0.35, dimmed: 0.1 } as const

const selectedModule = (): number | null => {
  const sel = nav().sel
  return sel?.kind === 'module' ? sel.id : null
}

const selectedImport = (): { from: number; to: number } | null => {
  const sel = nav().sel
  return sel?.kind === 'edge' ? sel : null
}

const SegmentMark: Component<{ box: SegmentBox; model: Model; layer: number }> = (props) => (
  <g>
    <rect
      x={props.box.x}
      y={props.box.y}
      width={props.box.w}
      height={props.box.h}
      rx="8"
      fill={layerTone(props.model, props.layer, LAYER_TONE.bandOff)}
      stroke={layerTone(props.model, props.layer, LAYER_TONE.tintEdge)}
    />
    <text class="arch-segment-name" x={props.box.x + 10} y={props.box.y + 17}>
      {props.box.segment.name === ROOT_SEGMENT ? t('arch.root') : `${props.box.segment.name}/`}
      <tspan class="arch-segment-meta" dx="8">
        {props.box.segment.modules.length}
      </tspan>
    </text>
  </g>
)

const ModuleMark: Component<{ node: ModuleNode; model: Model; unit: Unit; hasLabel: boolean }> = (props) => {
  const n = props.node
  const sev = worst(props.model.findingsOf(n.id))
  const isShared = props.model.isSharedByChosenFeatures(n.id)
  const isSelected = () => selectedModule() === n.id
  const stroke = () => {
    if (isSelected()) return SELECTED_STROKE
    if (sev) return SEV_COLOR[sev]
    return isShared ? SHARED_COLOR : layerTone(props.model, props.unit.layer, LAYER_TONE.tintEdge)
  }
  return (
    <g
      class="arch-module"
      data-testid="arch-module"
      data-id={n.id}
      data-shared={sharedAttr(props.model, n.id)}
      onClick={() => set({ sel: { kind: 'module', id: n.id } })}
      onDblClick={() => goToLevel(props.model, { level: 'code', container: props.unit.id, module: n.id })}
    >
      <rect
        x={n.x}
        y={n.y}
        width={n.w}
        height={n.h}
        rx="3"
        data-ch={!isSelected() && sev ? 'sev' : 'mod'}
        fill={NODE_FILL}
        stroke={stroke()}
        stroke-width={isSelected() ? 1.6 : 1}
      />
      <Show when={props.hasLabel}>
        <text class="arch-module-name" x={n.x + 6} y={n.y + 12.5}>
          {trunc(n.label, Math.floor((NODE.w - LABEL_PAD) / LABEL_CHAR_PX))}
        </text>
      </Show>
      <title>{(props.model.modules.get(n.id)?.path ?? '') + sharedNote(props.model, n.id)}</title>
    </g>
  )
}

const ImportMark: Component<{ arrow: ImportArrow; model: Model }> = (props) => {
  const a = props.arrow
  const sev = worst(props.model.findingsOnEdge(a.from, a.to))
  const isPicked = () => {
    const sel = selectedImport()
    return sel !== null && sel.from === a.from && sel.to === a.to
  }
  const opacity = () => {
    const focus = selectedModule()
    if (isPicked() || focus === a.from || focus === a.to) return ARROW_OPACITY.focused
    return focus === null ? ARROW_OPACITY.base : ARROW_OPACITY.dimmed
  }
  return (
    <path
      class="arch-import"
      data-testid="arch-import"
      data-from={a.from}
      data-to={a.to}
      data-ch={sev ? 'sev' : 'edge'}
      d={a.d}
      stroke={sev ? SEV_COLOR[sev] : EDGE_NEUTRAL}
      stroke-opacity={opacity()}
      stroke-width={isPicked() ? 2.2 : 1}
      marker-end={CANVAS_DEFS.arrow}
      onClick={() => set({ sel: { kind: 'edge', from: a.from, to: a.to } })}
    >
      <title>{`${props.model.modules.get(a.from)?.path ?? ''}\n→ ${props.model.modules.get(a.to)?.path ?? ''}`}</title>
    </path>
  )
}

const ComponentsLegend: Component = () => (
  <div class="legend map-legend">
    <div class="legend-row">
      <span class="map-lg map-lg-box" />
      {t('arch.legend.segment')}
    </div>
    <div class="legend-row">
      <span class="map-lg map-lg-box arch-lg-module" />
      {t('arch.legend.module')}
    </div>
    <div class="legend-row">
      <span class="map-lg map-lg-line" />
      {t('arch.legend.import')}
    </div>
    <div class="legend-row">
      <span class="map-lg" />
      {t('arch.legend.hint')}
    </div>
  </div>
)

export const ComponentsScene: Component<{ model: Model; unit: Unit }> = (props) => {
  const layout = createMemo(() => layoutComponents(props.model, props.unit))
  const marks = createMemo(() => marksWithinBudget(layout(), selectedModule()))
  return (
    <Canvas
      bounds={layout().bounds}
      fitKey={`components:${props.unit.id}`}
      legend={<ComponentsLegend />}
      overlay={
        <Show when={marks().hidden > 0}>
          <BudgetNote text={t('arch.budget.hidden', { n: marks().hidden })} />
        </Show>
      }
    >
      <g class="arch-components" data-container={props.unit.id}>
        <For each={layout().boxes}>
          {(box) => <SegmentMark box={box} model={props.model} layer={props.unit.layer} />}
        </For>
        <g class="arch-imports" fill="none">
          <For each={marks().segmentLinks}>
            {(link) => (
              <>
                <path
                  class="arch-segment-link"
                  data-ch="edge"
                  d={link.d}
                  stroke={EDGE_NEUTRAL}
                  stroke-opacity={ARROW_OPACITY.base}
                  stroke-width={1 + Math.min(3, Math.log2(link.count + 1) * 0.6)}
                  marker-end={CANVAS_DEFS.arrow}
                />
                <text class="map-count" x={link.label.x} y={link.label.y} fill={LABEL_FILL} stroke={LABEL_HALO}>
                  {link.count}
                </text>
              </>
            )}
          </For>
          <For each={marks().imports}>{(arrow) => <ImportMark arrow={arrow} model={props.model} />}</For>
        </g>
        <For each={layout().boxes}>
          {(box) => (
            <For each={box.nodes}>
              {(node) => <ModuleMark node={node} model={props.model} unit={props.unit} hasLabel={marks().hasLabels} />}
            </For>
          )}
        </For>
      </g>
    </Canvas>
  )
}
