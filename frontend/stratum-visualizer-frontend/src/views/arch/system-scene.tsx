import { Component, createMemo, For, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { trunc } from '../../render/labels'
import { CANVAS_DEFS, EDGE_NEUTRAL } from '../../render/encoding'
import { Model } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { LanesAside } from '../map/lanes-scene'
import { LanesLayout } from '../map/layout'
import { goToLevel } from './actions'
import { ARCH_CONTAINERS } from './path'
import { layoutPorts, layoutSystems, PortArrow, PortBox, SystemBox } from './system-layout'
import { SystemsInModel } from './systems'

const NAME_CHAR_PX = 6.6
const META_CHAR_PX = 5.6
const PAD = 12
const SELECTED_STROKE = 'oklch(94% 0.005 260)'
const SELF_FILL = 'oklch(34% 0.06 230 / .85)'
const SELF_STROKE = 'oklch(72% 0.1 230)'
const OTHER_FILL = 'oklch(28% 0.02 260 / .85)'
const OTHER_STROKE = 'oklch(62% 0.02 260)'
const MUTED_FILL = 'oklch(22% 0.008 260 / .6)'
const MUTED_STROKE = 'oklch(45% 0.01 260)'
const LABEL_FILL = 'oklch(90% 0.005 260)'
const LABEL_HALO = 'oklch(16% 0.012 260)'
const ARROW_WIDTH = { base: 1.6, selected: 2.8 } as const
const ARROW_OPACITY = { base: 0.6, selected: 0.95 } as const

const selectedSystem = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'system' ? sel.id : null
}

const selectedChannel = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'channel' ? sel.to : null
}

const selectedPort = (): { container: number; system: string } | null => {
  const sel = nav().sel
  return sel?.kind === 'port' ? sel : null
}

const boxFill = (b: SystemBox): { fill: string; stroke: string } => {
  if (b.isSelf) return { fill: SELF_FILL, stroke: SELF_STROKE }
  return b.system.unavailableReason ? { fill: MUTED_FILL, stroke: MUTED_STROKE } : { fill: OTHER_FILL, stroke: OTHER_STROKE }
}

const SystemMark: Component<{ box: SystemBox; model: Model }> = (props) => {
  const b = props.box
  const tone = boxFill(b)
  const isSelected = () => selectedSystem() === b.system.id
  const meta = () => {
    if (b.isSelf) return t('arch.system.self', { n: props.model.modules.size })
    if (b.system.unavailableReason) return t('arch.system.unavailable')
    return t('arch.system.contracts', { n: b.system.contractsTotal })
  }
  const title = () =>
    [b.system.name, b.system.unavailableReason, b.isSelf ? t('arch.system.enterHint') : null].filter(Boolean).join('\n')
  return (
    <g
      class={`arch-system${b.system.unavailableReason ? ' muted' : ''}`}
      data-testid="arch-system"
      data-id={b.system.id}
      data-self={b.isSelf ? 'true' : undefined}
      data-unavailable={b.system.unavailableReason ? 'true' : undefined}
      transform={`translate(${b.x},${b.y})`}
      onClick={() => set({ sel: { kind: 'system', id: b.system.id } })}
      onDblClick={() => b.isSelf && goToLevel(props.model, ARCH_CONTAINERS)}
    >
      <rect
        width={b.w}
        height={b.h}
        rx="10"
        fill={tone.fill}
        stroke={isSelected() ? SELECTED_STROKE : tone.stroke}
        stroke-width={isSelected() ? 2 : 1.2}
        stroke-dasharray={b.system.unavailableReason ? '5 3' : undefined}
      />
      <text class="arch-system-name" x={PAD} y={24}>
        {trunc(b.system.name, Math.floor((b.w - PAD * 2) / NAME_CHAR_PX))}
      </text>
      <text class="arch-system-meta" x={PAD} y={42}>
        {meta()}
      </text>
      <Show when={b.system.unavailableReason}>
        {(reason) => (
          <text class="arch-system-reason" x={PAD} y={58}>
            {trunc(reason(), Math.floor((b.w - PAD * 2) / META_CHAR_PX))}
          </text>
        )}
      </Show>
      <title>{title()}</title>
    </g>
  )
}

const ArrowLabel: Component<{ x: number; y: number; n: number }> = (props) => (
  <text class="map-count" x={props.x} y={props.y} fill={LABEL_FILL} stroke={LABEL_HALO}>
    {props.n}
  </text>
)

export const SystemScene: Component<{ model: Model; systems: SystemsInModel }> = (props) => {
  const layout = createMemo(() => layoutSystems(props.systems))
  return (
    <Canvas bounds={layout().bounds} fitKey="arch-system">
      <g class="arch-systems">
        <g fill="none">
          <For each={layout().arrows}>
            {(a) => {
              const isSelected = () => selectedChannel() === a.channel.to.id
              return (
                <>
                  <path
                    class="arch-channel"
                    data-testid="arch-channel"
                    data-from={props.systems.self.id}
                    data-to={a.channel.to.id}
                    d={a.d}
                    stroke={isSelected() ? SELECTED_STROKE : EDGE_NEUTRAL}
                    stroke-opacity={isSelected() ? ARROW_OPACITY.selected : ARROW_OPACITY.base}
                    stroke-width={isSelected() ? ARROW_WIDTH.selected : ARROW_WIDTH.base}
                    marker-end={CANVAS_DEFS.arrow}
                    onClick={() => set({ sel: { kind: 'channel', to: a.channel.to.id } })}
                  >
                    <title>
                      {props.systems.self.name} → {a.channel.to.name}
                      {'\n'}
                      {t('arch.channel.contracts', { n: a.channel.contracts.length })}
                    </title>
                  </path>
                  <ArrowLabel x={a.label.x} y={a.label.y} n={a.channel.contracts.length} />
                </>
              )
            }}
          </For>
        </g>
        <For each={layout().boxes}>{(b) => <SystemMark box={b} model={props.model} />}</For>
      </g>
    </Canvas>
  )
}

const PortMark: Component<{ port: PortBox }> = (props) => {
  const p = props.port
  const isSelected = () => selectedSystem() === p.system.id
  const tone = p.system.unavailableReason ? { fill: MUTED_FILL, stroke: MUTED_STROKE } : { fill: OTHER_FILL, stroke: OTHER_STROKE }
  return (
    <g
      class={`arch-port${p.system.unavailableReason ? ' muted' : ''}`}
      data-testid="arch-port"
      data-id={p.system.id}
      transform={`translate(${p.x},${p.y})`}
      onClick={() => set({ sel: { kind: 'system', id: p.system.id } })}
    >
      <rect
        width={p.w}
        height={p.h}
        rx={p.h / 2}
        fill={tone.fill}
        stroke={isSelected() ? SELECTED_STROKE : tone.stroke}
        stroke-width={isSelected() ? 2 : 1.2}
        stroke-dasharray={p.system.unavailableReason ? '5 3' : undefined}
      />
      <text class="arch-system-name" x={PAD + 4} y={20}>
        {trunc(p.system.name, Math.floor((p.w - PAD * 2) / NAME_CHAR_PX))}
      </text>
      <text class="arch-system-meta" x={PAD + 4} y={34}>
        {p.system.unavailableReason ? t('arch.system.unavailable') : t('arch.port')}
      </text>
      <title>{[p.system.name, p.system.unavailableReason].filter(Boolean).join('\n')}</title>
    </g>
  )
}

const PortArrowMark: Component<{ arrow: PortArrow; model: Model }> = (props) => {
  const link = props.arrow.link
  const isSelected = () => {
    const p = selectedPort()
    return (p?.container === link.container && p.system === link.system) || selectedSystem() === link.system
  }
  return (
    <>
      <path
        class="arch-port-link"
        data-testid="arch-port-link"
        data-container={link.container}
        data-system={link.system}
        d={props.arrow.d}
        stroke={isSelected() ? SELECTED_STROKE : EDGE_NEUTRAL}
        stroke-opacity={isSelected() ? ARROW_OPACITY.selected : ARROW_OPACITY.base / 2}
        stroke-width={isSelected() ? ARROW_WIDTH.selected : ARROW_WIDTH.base}
        stroke-dasharray="6 3"
        marker-end={CANVAS_DEFS.arrow}
        onClick={() => set({ sel: { kind: 'port', container: link.container, system: link.system } })}
      >
        <title>
          {props.model.unitLabel(link.container)} → {link.system}
          {'\n'}
          {t('arch.channel.contracts', { n: link.contracts.length })}
        </title>
      </path>
      <Show when={isSelected()}>
        <ArrowLabel x={props.arrow.label.x} y={props.arrow.label.y} n={link.contracts.length} />
      </Show>
    </>
  )
}

export const portsAside = (m: Model, systems: SystemsInModel): LanesAside => {
  const portsOf = (layout: LanesLayout) => layoutPorts(layout, systems)
  return {
    widthOf: (layout) => portsOf(layout).width,
    Marks: (props) => {
      const ports = createMemo(() => portsOf(props.layout))
      return (
        <g class="arch-ports">
          <g fill="none">
            <For each={ports().arrows}>{(a) => <PortArrowMark arrow={a} model={m} />}</For>
          </g>
          <For each={ports().ports}>{(p) => <PortMark port={p} />}</For>
        </g>
      )
    },
  }
}
