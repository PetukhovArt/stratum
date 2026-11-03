import { Component, createMemo, For, Show } from 'solid-js'
import { BudgetNote } from '../../canvas/budget-note'
import { Canvas } from '../../canvas/canvas'
import { sharedAttr, sharedNote } from '../../features/shared-mark'
import { t } from '../../i18n'
import { Edge, EDGE_SELECTED, EdgeState } from '../../render/edge'
import { LAYER_TONE, SEV_COLOR, SHARED_COLOR } from '../../render/encoding'
import { Model, Unit, worst } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { layerTone } from '../focus/paint'
import { goToLevel } from './actions'
import { ImportArrow, componentMarks, layoutComponents, ModuleNode, ROOT_SEGMENT, SegmentBox } from './components'

const LABEL_BASELINE = 13.5
const SELECTED_STROKE = EDGE_SELECTED
const NODE_FILL = 'oklch(30% 0.02 260)'
const ARROW_OPACITY = 0.35
const IMPORT_WIDTH = 1
const segmentWidth = (count: number): number => 1 + Math.min(1.5, Math.log2(count + 1) * 0.5)
const NEIGHBOUR = { imports: 'oklch(74% 0.13 235)', importedBy: 'oklch(80% 0.13 75)' } as const

type Role = 'selected' | 'imports' | 'importedBy' | 'other' | 'idle'

const roleOf = (m: Model, id: number): Role => {
  const sel = selectedModule()
  if (sel === null) return 'idle'
  if (sel === id) return 'selected'
  if (m.out(sel).includes(id)) return 'imports'
  if (m.inn(sel).includes(id)) return 'importedBy'
  return 'other'
}

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
  const role = () => roleOf(props.model, n.id)
  const isSelected = () => role() === 'selected'
  const stroke = () => {
    const r = role()
    if (r === 'selected') return SELECTED_STROKE
    if (r === 'imports' || r === 'importedBy') return NEIGHBOUR[r]
    if (sev) return SEV_COLOR[sev]
    return isShared ? SHARED_COLOR : layerTone(props.model, props.unit.layer, LAYER_TONE.tintEdge)
  }
  return (
    <g
      class="arch-module"
      data-testid="arch-module"
      data-id={n.id}
      data-shared={sharedAttr(props.model, n.id)}
      data-role={role()}
      opacity={role() === 'other' ? 0.35 : 1}
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
        stroke-width={role() === 'idle' || role() === 'other' ? 1 : 1.8}
      />
      <Show when={props.hasLabel}>
        <text class="arch-module-name" x={n.x + 6} y={n.y + LABEL_BASELINE}>
          {n.name}
        </text>
      </Show>
      <title>{(props.model.modules.get(n.id)?.path ?? '') + sharedNote(props.model, n.id)}</title>
    </g>
  )
}

const ImportMark: Component<{ arrow: ImportArrow; model: Model }> = (props) => {
  const a = props.arrow
  const sev = worst(props.model.findingsOnEdge(a.from, a.to))
  const state = (): EdgeState => {
    const sel = selectedImport()
    if (sel !== null && sel.from === a.from && sel.to === a.to) return 'selected'
    const focus = selectedModule()
    if (focus === a.from || focus === a.to) return 'focused'
    return focus === null ? 'idle' : 'dimmed'
  }
  const colour = () => {
    const focus = selectedModule()
    if (focus === a.from) return NEIGHBOUR.imports
    return focus === a.to ? NEIGHBOUR.importedBy : undefined
  }
  return (
    <Edge
      d={a.d}
      kind={sev ? 'break' : 'neutral'}
      severity={sev}
      color={colour()}
      width={IMPORT_WIDTH}
      opacity={ARROW_OPACITY}
      state={state()}
      testId="arch-import"
      class="arch-import"
      data={{ 'data-from': a.from, 'data-to': a.to }}
      onClick={() => set({ sel: { kind: 'edge', from: a.from, to: a.to } })}
      title={`${props.model.modules.get(a.from)?.path ?? ''}\n→ ${props.model.modules.get(a.to)?.path ?? ''}`}
    />
  )
}

const ComponentsLegend: Component = () => (
  <div class="legend">
    <div class="legend-row">
      <span class="arch-lg arch-lg-box" />
      {t('arch.legend.segment')}
    </div>
    <div class="legend-row">
      <span class="arch-lg arch-lg-box arch-lg-module" />
      {t('arch.legend.module')}
    </div>
    <div class="legend-row">
      <span class="arch-lg arch-lg-line" />
      {t('arch.legend.import')}
    </div>
    <div class="legend-row">
      <span class="arch-lg arch-lg-box" style={{ 'border-color': NEIGHBOUR.imports }} />
      <span class="arch-lg arch-lg-box" style={{ 'border-color': NEIGHBOUR.importedBy }} />
      {t('arch.legend.neighbours')}
    </div>
    <div class="legend-row">
      <span class="arch-lg" />
      {t('arch.legend.hint')}
    </div>
  </div>
)

export const ComponentsScene: Component<{ model: Model; unit: Unit }> = (props) => {
  const layout = createMemo(() => layoutComponents(props.model, props.unit))
  const marks = createMemo(() =>
    componentMarks(layout(), selectedModule(), (a) => props.model.findingsOnEdge(a.from, a.to).length > 0),
  )
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
              <Edge
                d={link.d}
                width={segmentWidth(link.count)}
                opacity={ARROW_OPACITY}
                state={selectedModule() === null ? 'idle' : 'dimmed'}
                class="arch-segment-link"
                data={{ 'data-from': link.from, 'data-to': link.to }}
                label={{ ...link.label, text: link.count }}
              />
            )}
          </For>
        </g>
        <g class="arch-imports" fill="none">
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
