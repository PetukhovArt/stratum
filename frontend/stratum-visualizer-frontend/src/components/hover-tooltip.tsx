import { Component, createMemo, For, Match, Show, Switch } from 'solid-js'
import { DesignData, DesignModule, layerOfModule, violationsOf } from '../render/design'
import { layerColor, stageInfo, TONE } from '../render/palette'
import { HoveredEdge, HoveredMod } from '../state'

const CURSOR_OFFSET_PX = 14
const EDGE_TIP_WIDTH_PX = 320
const MODULE_TIP_WIDTH_PX = 260
const TIP_EST_HEIGHT_PX = 160
const CANVAS_MARGIN_PX = 8
const MAX_LISTED_VIOLATIONS = 3

const EdgeTooltip: Component<{ edge: HoveredEdge; data: DesignData }> = (props) => {
  const label = (id: string) => props.data.moduleById.get(id)?.label ?? id
  const vio = () => (props.edge.violation ? props.data.violationById.get(props.edge.violation) : undefined)
  return (
    <>
      <div class="tt-hd">
        <span class={`tt-kind tt-kind-${props.edge.kind}`}>{props.edge.kind}</span>
        <span class="tt-edge-arrow">→</span>
        <Show when={vio()}>{(v) => <span class={`tt-sev sev-${v().severity}`}>{v().severity}</span>}</Show>
      </div>
      <div class="tt-edge-route">
        <div class="tt-edge-end">{label(props.edge.source)}</div>
        <div class="tt-edge-end">↓ {label(props.edge.target)}</div>
      </div>
      <Show when={vio()}>
        {(v) => (
          <div class="tt-vio">
            <div class="tt-vio-rule">{v().rule}</div>
            <div class="tt-vio-msg">{v().message}</div>
          </div>
        )}
      </Show>
    </>
  )
}

const ModuleTooltip: Component<{ mod: DesignModule; data: DesignData }> = (props) => {
  const m = () => props.mod
  const layer = () => layerOfModule(props.data, m())
  const vios = createMemo(() => violationsOf(props.data, m()))
  const stage = () => stageInfo(m().stage)
  const kidsCount = () => props.data.childIndex[m().id]?.length ?? 0
  const inCount = createMemo(() => props.data.edges.filter((e) => e.target === m().id).length)
  const outCount = createMemo(() => props.data.edges.filter((e) => e.source === m().id).length)

  return (
    <>
      <div class="tt-hd">
        <Show when={layer()}>
          {(l) => (
            <span
              class="tt-layer"
              style={{
                background: layerColor(l().hue, TONE.chipBg),
                color: layerColor(l().hue, TONE.chipText),
              }}
            >
              {l().label}
            </span>
          )}
        </Show>
        <span class={`tt-stage tt-stage-${m().stage}`} title={stage().desc}>
          ◔ {stage().name}
        </span>
        <Show when={m().kind === 'composer'}>
          <span class="tt-kind tt-kind-composer">◇ composer</span>
        </Show>
        <Show when={m().kind === 'shared'}>
          <span class="tt-kind tt-kind-shared">_shared</span>
        </Show>
        <Show when={m().severity}>
          <span class={`tt-sev sev-${m().severity}`}>{m().severity}</span>
        </Show>
      </div>
      <div class="tt-name">{m().label}</div>
      <div class="tt-path">{m().path}</div>
      <div class="tt-stats">
        <span title={stage().desc}>
          stage {m().stage} · {stage().name}
        </span>
        <span>in {inCount()}</span>
        <span>out {outCount()}</span>
        <Show when={kidsCount() > 0}>
          <span>{kidsCount()} children</span>
        </Show>
      </div>
      <Show when={vios().length > 0}>
        <div class="tt-vios">
          <For each={vios().slice(0, MAX_LISTED_VIOLATIONS)}>
            {(v) => (
              <div class={`tt-vio-row sev-${v.severity}`}>
                <span class={`tt-vio-tag sev-${v.severity}`}>{v.severity}</span>
                <span class="tt-vio-rule">{v.rule}</span>
              </div>
            )}
          </For>
          <Show when={vios().length > MAX_LISTED_VIOLATIONS}>
            <div class="tt-vio-more">
              +{vios().length - MAX_LISTED_VIOLATIONS} more · click to inspect
            </div>
          </Show>
        </div>
      </Show>
    </>
  )
}

export const HoverTooltip: Component<{
  data: DesignData
  hoveredModule: HoveredMod | null
  hoveredEdge: HoveredEdge | null
  canvasRect: DOMRect | null
}> = (props) => {
  const hoveredMod = () => {
    const h = props.hoveredModule
    return h ? props.data.moduleById.get(h.id) : undefined
  }
  const anchor = () => props.hoveredModule ?? props.hoveredEdge
  const width = () => (props.hoveredModule ? MODULE_TIP_WIDTH_PX : EDGE_TIP_WIDTH_PX)
  const position = () => {
    const a = anchor()
    const rect = props.canvasRect
    if (!a || !rect) return null
    return {
      left: Math.min(a.x - rect.left + CURSOR_OFFSET_PX, rect.width - width() - CANVAS_MARGIN_PX),
      top: Math.min(a.y - rect.top + CURSOR_OFFSET_PX, rect.height - TIP_EST_HEIGHT_PX),
    }
  }

  return (
    <Show when={position()}>
      {(p) => (
        <div class="tt" style={{ left: `${p().left}px`, top: `${p().top}px`, width: `${width()}px` }}>
          <Switch>
            <Match when={hoveredMod()}>{(m) => <ModuleTooltip mod={m()} data={props.data} />}</Match>
            <Match when={props.hoveredEdge}>
              {(e) => <EdgeTooltip edge={e()} data={props.data} />}
            </Match>
          </Switch>
        </div>
      )}
    </Show>
  )
}
