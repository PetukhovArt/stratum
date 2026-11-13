import { Component, createMemo, For, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { Edge } from '../../render/edge'
import { trunc } from '../../render/labels'
import { Model } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { goToLevel } from './actions'
import { ARCH_CONTAINERS } from './path'
import { ActorBox, layoutSystems, LinkArrow, SystemBox, Zone, zoneTitleBelow } from './system-layout'
import { ChannelInUse, SystemsInModel } from './systems'

const NAME_CHAR_PX = 6.6
const META_CHAR_PX = 5.6
const PAD = 12
const SELECTED_STROKE = 'oklch(94% 0.005 260)'
const SELF_FILL = 'oklch(34% 0.06 230 / .85)'
const SELF_STROKE = 'oklch(72% 0.1 230)'
const OTHER_FILL = 'oklch(28% 0.02 260 / .85)'
const OTHER_STROKE = 'oklch(62% 0.02 260)'
const EXTERNAL_FILL = 'oklch(30% 0.03 310 / .85)'
const EXTERNAL_STROKE = 'oklch(66% 0.05 310)'
const ACTOR_FILL = 'oklch(30% 0.07 255 / .9)'
const ACTOR_STROKE = 'oklch(70% 0.1 255)'
const ZONE_TONE: Record<Zone['kind'], string> = {
  actors: 'oklch(70% 0.1 255)',
  system: 'oklch(72% 0.1 230)',
  external: 'oklch(66% 0.05 310)',
}
const LINK_CHARS = 30
const DESC_LINES = 2

const wrapped = (text: string, chars: number, max: number): string[] => {
  const lines: string[] = []
  let line = ''
  for (const word of text.split(/\s+/).filter(Boolean)) {
    if (line && line.length + 1 + word.length > chars) {
      lines.push(line)
      line = word
    } else line = line ? `${line} ${word}` : word
  }
  if (line) lines.push(line)
  if (lines.length <= max) return lines.map((l) => trunc(l, chars))
  return [...lines.slice(0, max - 1), trunc(lines.slice(max - 1).join(' '), chars)]
}
const MUTED_FILL = 'oklch(22% 0.008 260 / .6)'
const MUTED_STROKE = 'oklch(45% 0.01 260)'
const ARROW_WIDTH = 1.6
const LINK_WIDTH = 1.1
const LINK_DASH = '2 4'
const LINK_OPACITY = 0.42
const LINK_LABEL = 'oklch(76% 0.012 260)'
const EXTERNAL_DASH = '6 3'
const TAG = { h: 16, charPx: 6, pad: 7, top: 10 } as const
const LINE_TOP = 43
const LINE_STEP = 17

const channelText = (c: ChannelInUse): string =>
  [
    c.declared ? trunc(c.declared, LINK_CHARS) : null,
    c.contracts.length || null,
    c.via.length ? t('arch.channel.via', { n: c.via.length }) : null,
  ]
    .filter((part) => part !== null)
    .join(' · ')

const channelTitle = (self: string, c: ChannelInUse): string =>
  [
    `${self} → ${c.to.name}`,
    c.declared,
    c.contracts.length ? t('arch.channel.contracts', { n: c.contracts.length }) : null,
    c.via.length ? t('arch.channel.via', { n: c.via.length }) : null,
  ]
    .filter(Boolean)
    .join('\n')

const ExternalTag: Component<{ boxW: number }> = (props) => {
  const text = t('arch.system.externalTag')
  const w = text.length * TAG.charPx + TAG.pad * 2
  return (
    <g class="arch-system-tag" data-testid="arch-system-external" transform={`translate(${props.boxW - PAD - w},${TAG.top})`}>
      <rect width={w} height={TAG.h} rx={TAG.h / 2} fill="none" stroke={EXTERNAL_STROKE} stroke-dasharray="3 2" />
      <text x={w / 2} y={TAG.h / 2}>
        {text}
      </text>
    </g>
  )
}

const tagWidth = (): number => t('arch.system.externalTag').length * TAG.charPx + TAG.pad * 2 + 6

const selectedSystem = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'system' ? sel.id : null
}

const selectedChannel = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'channel' ? sel.to : null
}

const boxFill = (b: SystemBox): { fill: string; stroke: string } => {
  if (b.isSelf) return { fill: SELF_FILL, stroke: SELF_STROKE }
  if (b.system.isExternal) return { fill: EXTERNAL_FILL, stroke: EXTERNAL_STROKE }
  return b.system.unavailableReason ? { fill: MUTED_FILL, stroke: MUTED_STROKE } : { fill: OTHER_FILL, stroke: OTHER_STROKE }
}

const SystemMark: Component<{ box: SystemBox; model: Model }> = (props) => {
  const b = props.box
  const tone = boxFill(b)
  const isSelected = () => selectedSystem() === b.system.id
  const meta = () => {
    if (b.isSelf) return t('arch.system.self', { n: props.model.modules.size })
    if (b.system.unavailableReason) return t('arch.system.unavailable')
    if (b.system.contractsTotal) return t('arch.system.contracts', { n: b.system.contractsTotal })
    return b.system.isExternal ? null : t('arch.system.internal')
  }
  const detail = () => b.system.unavailableReason ?? b.system.description
  const lines = () =>
    [
      { class: 'arch-system-meta', text: meta() },
      { class: 'arch-system-reason', text: detail() },
    ].filter((line): line is { class: string; text: string } => line.text !== null)
  const title = () =>
    [b.system.name, b.system.description, b.system.unavailableReason, b.isSelf ? t('arch.system.enterHint') : null]
      .filter(Boolean)
      .join('\n')
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
        stroke-dasharray={b.system.unavailableReason ? '5 3' : b.system.isExternal ? EXTERNAL_DASH : undefined}
      />
      <text class="arch-system-name" x={PAD} y={24}>
        {trunc(b.system.name, Math.floor((b.w - PAD * 2 - (b.system.isExternal ? tagWidth() : 0)) / NAME_CHAR_PX))}
      </text>
      <Show when={b.system.isExternal}>
        <ExternalTag boxW={b.w} />
      </Show>
      <For each={lines()}>
        {(line, i) => (
          <text class={line.class} x={PAD} y={LINE_TOP + i() * LINE_STEP}>
            {trunc(line.text, Math.floor((b.w - PAD * 2) / META_CHAR_PX))}
          </text>
        )}
      </For>
      <title>{title()}</title>
    </g>
  )
}

const ZoneMark: Component<{ zone: Zone }> = (props) => (
  <g class="arch-zone" data-testid="arch-zone" data-kind={props.zone.kind}>
    <rect
      x={props.zone.x}
      y={props.zone.y}
      width={props.zone.w}
      height={props.zone.h}
      rx="14"
      fill={ZONE_TONE[props.zone.kind]}
      fill-opacity="0.05"
      stroke={ZONE_TONE[props.zone.kind]}
      stroke-opacity="0.45"
      stroke-dasharray="6 4"
    />
    <text
      class="arch-zone-name"
      x={props.zone.x + 14}
      y={zoneTitleBelow(props.zone.kind) ? props.zone.y + props.zone.h - 11 : props.zone.y + 20}
      fill={ZONE_TONE[props.zone.kind]}
    >
      {t(`arch.zone.${props.zone.kind}`)}
    </text>
  </g>
)

const ActorMark: Component<{ box: ActorBox }> = (props) => {
  const b = props.box
  return (
    <g class="arch-actor" data-testid="arch-actor" data-id={b.actor.id} transform={`translate(${b.x},${b.y})`}>
      <rect width={b.w} height={b.h} rx="10" fill={ACTOR_FILL} stroke={ACTOR_STROKE} stroke-width="1.2" />
      <circle cx={PAD + 9} cy={20} r="6" fill="none" stroke={ACTOR_STROKE} stroke-width="1.4" />
      <path d={`M${PAD},${36} q9,-12 18,0`} fill="none" stroke={ACTOR_STROKE} stroke-width="1.4" />
      <text class="arch-system-name" x={PAD + 26} y={26}>
        {trunc(b.actor.name, Math.floor((b.w - PAD * 2 - 26) / NAME_CHAR_PX))}
      </text>
      <For each={wrapped(b.actor.description ?? '', Math.floor((b.w - PAD * 2) / META_CHAR_PX), DESC_LINES)}>
        {(line, i) => (
          <text class="arch-system-reason" x={PAD} y={LINE_TOP + 4 + i() * (LINE_STEP - 3)}>
            {line}
          </text>
        )}
      </For>
      <title>{[b.actor.name, b.actor.description].filter(Boolean).join('\n')}</title>
    </g>
  )
}

const LinkMark: Component<{ arrow: LinkArrow; nameOf: (id: string) => string }> = (props) => {
  const a = props.arrow
  return (
    <Edge
      d={a.d}
      width={LINK_WIDTH}
      dash={LINK_DASH}
      opacity={LINK_OPACITY}
      testId="arch-link"
      class="arch-link"
      data={{ 'data-from': a.link.from, 'data-to': a.link.to }}
      label={
        a.link.label
          ? { ...a.label, text: trunc(a.link.label, LINK_CHARS), class: 'edge-label', color: LINK_LABEL }
          : null
      }
      title={[`${props.nameOf(a.link.from)} → ${props.nameOf(a.link.to)}`, a.link.label, t('arch.link.declared')]
        .filter(Boolean)
        .join('\n')}
    />
  )
}

export const SystemScene: Component<{ model: Model; systems: SystemsInModel }> = (props) => {
  const layout = createMemo(() => layoutSystems(props.systems))
  const nameOf = (id: string): string =>
    props.systems.actors.find((a) => a.id === id)?.name ??
    [props.systems.self, ...props.systems.others].find((s) => s.id === id)?.name ??
    id
  return (
    <Canvas bounds={layout().bounds} fitKey="arch-system">
      <g class="arch-systems">
        <For each={layout().zones}>{(z) => <ZoneMark zone={z} />}</For>
        <g fill="none">
          <For each={layout().links}>{(a) => <LinkMark arrow={a} nameOf={nameOf} />}</For>
          <For each={layout().arrows}>
            {(a) => (
              <Edge
                d={a.d}
                width={ARROW_WIDTH}
                state={selectedChannel() === a.channel.to.id ? 'selected' : 'idle'}
                testId="arch-channel"
                class="arch-channel"
                data={{
                  'data-from': props.systems.self.id,
                  'data-to': a.channel.to.id,
                  'data-contracts': a.channel.contracts.length,
                  'data-via': a.channel.via.length,
                }}
                label={{ ...a.label, text: channelText(a.channel) }}
                onClick={() => set({ sel: { kind: 'channel', to: a.channel.to.id } })}
                title={channelTitle(props.systems.self.name, a.channel)}
              />
            )}
          </For>
        </g>
        <For each={layout().actors}>{(b) => <ActorMark box={b} />}</For>
        <For each={layout().boxes}>{(b) => <SystemMark box={b} model={props.model} />}</For>
      </g>
    </Canvas>
  )
}
