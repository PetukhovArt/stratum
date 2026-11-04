import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { BudgetNote } from '../../canvas/budget-note'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { Edge, EDGE_SELECTED, EdgeState } from '../../render/edge'
import { trunc } from '../../render/labels'
import { Model } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { openBox } from './actions'
import { Box, BoxLink, Boxes } from './boxes'
import {
  AreaFrame,
  layerRanks,
  layoutBoxes,
  Orient,
  PlacedBox,
  PlacedStub,
  PlacedFlow,
  PlacedLink,
  PlacedPort,
  PortFlow,
  PortTarget,
  routeLinks,
  shownLinks,
  shownStubs,
} from './boxes-layout'
import { ArchGroup } from './path'
import { channelWeight, systemsIn } from './systems'

const NAME_CHAR_PX = 7
const PAD = 12
const FILL: Readonly<Record<Box['kind'], string>> = {
  feature: 'oklch(36% 0.07 235 / .9)',
  layer: 'oklch(31% 0.03 260 / .9)',
  unit: 'oklch(34% 0.05 235 / .9)',
}
const STROKE: Readonly<Record<Box['kind'], string>> = {
  feature: 'oklch(72% 0.1 235)',
  layer: 'oklch(64% 0.03 260)',
  unit: 'oklch(70% 0.08 235)',
}
const PORT_FILL = { internal: 'oklch(28% 0.02 260 / .9)', external: 'oklch(30% 0.03 310 / .9)' } as const
const PORT_STROKE = { internal: 'oklch(62% 0.02 260)', external: 'oklch(66% 0.05 310)' } as const
const STUB_FILL = 'oklch(24% 0.012 260 / .9)'
const STUB_CHAR_PX = 6.2
const STUB_WIDTH = 1.2
const FLOW_WIDTH = 1.3
const FLOW_OPACITY = 0.25
const AREA_STROKE = 'oklch(60% 0.02 260 / .35)'

const ORIENT_KEY = 'stratum.arch.boxesOrient'
const ORIENTS: readonly Orient[] = ['rows', 'columns']
const [orient, setOrientSignal] = createSignal<Orient>(localStorage.getItem(ORIENT_KEY) === 'columns' ? 'columns' : 'rows')

const setOrient = (next: Orient): void => {
  setOrientSignal(next)
  localStorage.setItem(ORIENT_KEY, next)
}

const OrientToggle: Component = () => (
  <div class="arch-boxes-orient" data-testid="arch-boxes-orient" role="group" aria-label={t('arch.boxes.orient')}>
    <For each={ORIENTS}>
      {(o) => (
        <button
          data-orient={o}
          aria-pressed={orient() === o}
          classList={{ on: orient() === o }}
          title={t(o === 'rows' ? 'arch.boxes.rowsTitle' : 'arch.boxes.columnsTitle')}
          onClick={() => setOrient(o)}
        >
          {t(o === 'rows' ? 'arch.boxes.rows' : 'arch.boxes.columns')}
        </button>
      )}
    </For>
  </div>
)

const selectedBox = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'box' ? sel.key : null
}

const selectedLink = (): { from: string; to: string } | null => {
  const sel = nav().sel
  return sel?.kind === 'link' ? sel : null
}

const isSelectedLink = (l: BoxLink): boolean => {
  const sel = selectedLink()
  return sel !== null && sel.from === l.from && sel.to === l.to
}

const selectedPort = (): { box: string; system: string } | null => {
  const sel = nav().sel
  return sel?.kind === 'port' ? sel : null
}

const selectedSystem = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'system' ? sel.id : null
}

const selectLink = (l: BoxLink): void => set({ sel: { kind: 'link', from: l.from, to: l.to } })

const linkTitle = (l: BoxLink, names: (key: string) => string, hint: string): string =>
  [
    `${names(l.from)} → ${names(l.to)}`,
    t('arch.box.imports', { n: l.count }),
    l.findings ? t('arch.box.findings', { n: l.findings }) : null,
    hint,
  ]
    .filter(Boolean)
    .join('\n')

const BoxMark: Component<{ placed: PlacedBox; isDimmed: boolean; onOpen: () => void }> = (props) => {
  const p = props.placed
  const isSelected = () => selectedBox() === p.box.key
  const meta = () =>
    p.usedBy > 0
      ? `${t('arch.box.modules', { n: p.box.modules })} · ${t('arch.boxes.usedBy', { n: p.usedBy })}`
      : p.box.units.length > 1
        ? t('arch.box.meta', { modules: p.box.modules, units: p.box.units.length })
        : t('arch.box.modules', { n: p.box.modules })
  return (
    <g
      class="arch-box"
      data-testid="arch-box"
      data-key={p.box.key}
      data-kind={p.box.kind}
      transform={`translate(${p.x},${p.y})`}
      opacity={props.isDimmed ? 0.4 : 1}
      onClick={() => set({ sel: { kind: 'box', key: p.box.key } })}
      onDblClick={() => props.onOpen()}
    >
      <rect
        width={p.w}
        height={p.h}
        rx="9"
        fill={FILL[p.box.kind]}
        stroke={isSelected() ? EDGE_SELECTED : STROKE[p.box.kind]}
        stroke-width={isSelected() ? 2.2 : 1.2}
      />
      <text class="arch-box-name" x={PAD} y={24}>
        {trunc(p.box.name, Math.floor((p.w - PAD * 2) / NAME_CHAR_PX))}
      </text>
      <text class="arch-box-meta" x={PAD} y={42}>
        {meta()}
      </text>
      <title>
        {p.box.name}
        {'\n'}
        {meta()}
        {'\n'}
        {t('arch.box.openHint')}
      </title>
    </g>
  )
}

const LinkMark: Component<{ placed: PlacedLink; focus: string | null; names: (key: string) => string }> = (props) => {
  const l = props.placed.link
  const state = (): EdgeState => {
    if (isSelectedLink(l)) return 'selected'
    return props.focus === l.from || props.focus === l.to ? 'focused' : 'idle'
  }
  return (
    <Edge
      d={props.placed.d}
      kind={l.breaks && l.worst ? 'break' : 'neutral'}
      severity={l.worst}
      weight={l.count}
      state={state()}
      label={{ ...props.placed.label, text: l.count }}
      testId="arch-box-link"
      class="arch-box-link"
      data={{ 'data-from': l.from, 'data-to': l.to, 'data-count': l.count }}
      onClick={() => selectLink(l)}
      title={linkTitle(l, props.names, t('arch.box.linkHint'))}
    />
  )
}

const StubMark: Component<{ stub: PlacedStub; names: (key: string) => string }> = (props) => {
  const s = props.stub
  const text = `${s.hubName} ${s.link.count}`
  const isSelected = () => isSelectedLink(s.link)
  return (
    <Edge
      d={s.d}
      width={STUB_WIDTH}
      state={isSelected() ? 'selected' : 'idle'}
      testId="arch-box-stub"
      class="arch-box-stub"
      data={{ 'data-from': s.link.from, 'data-to': s.link.to, 'data-count': s.link.count }}
      onClick={() => selectLink(s.link)}
      title={linkTitle(s.link, props.names, t('arch.boxes.stubHint'))}
    >
      <rect
        x={s.x}
        y={s.y}
        width={s.w}
        height={s.h}
        rx="4"
        fill={STUB_FILL}
        stroke={isSelected() ? EDGE_SELECTED : STROKE.layer}
        stroke-width={isSelected() ? 1.6 : 1}
        stroke-dasharray="3 2"
      />
      <text class="arch-stub-name" x={s.x + 6} y={s.y + s.h / 2 + 3.5}>
        {trunc(text, Math.floor((s.w - 10) / STUB_CHAR_PX))}
      </text>
    </Edge>
  )
}

const AreaMark: Component<{ area: AreaFrame }> = (props) => (
  <g class="arch-area">
    <rect x={props.area.x} y={props.area.y} width={props.area.w} height={props.area.h} rx="12" fill="none" stroke={AREA_STROKE} />
    <text class="arch-area-name" x={props.area.x + 8} y={props.area.y - 4}>
      {props.area.name}
    </text>
  </g>
)

const PortMark: Component<{ placed: PlacedPort }> = (props) => {
  const p = props.placed
  const tone = p.port.isExternal ? 'external' : 'internal'
  const isSelected = () => selectedSystem() === p.port.id
  return (
    <g
      class="arch-port"
      data-testid="arch-port"
      data-id={p.port.id}
      transform={`translate(${p.x},${p.y})`}
      onClick={() => set({ sel: { kind: 'system', id: p.port.id } })}
    >
      <rect
        width={p.w}
        height={p.h}
        rx={p.h / 2}
        fill={PORT_FILL[tone]}
        stroke={isSelected() ? EDGE_SELECTED : PORT_STROKE[tone]}
        stroke-width={isSelected() ? 2 : 1.2}
      />
      <text class="arch-box-name" x={PAD + 6} y={20}>
        {trunc(p.port.name, Math.floor((p.w - PAD * 2) / NAME_CHAR_PX))}
      </text>
      <text class="arch-box-meta" x={PAD + 6} y={35}>
        {t(p.port.isExternal ? 'arch.system.external' : 'arch.port')}
      </text>
      <title>{p.port.name}</title>
    </g>
  )
}

const FlowMark: Component<{ placed: PlacedFlow; focus: string | null; names: (key: string) => string }> = (props) => {
  const f = props.placed.flow
  const state = (): EdgeState => {
    const sel = selectedPort()
    if ((sel?.box === f.box && sel.system === f.port) || selectedSystem() === f.port) return 'selected'
    return props.focus === f.box ? 'focused' : 'idle'
  }
  return (
    <Edge
      d={props.placed.d}
      kind="flow"
      width={FLOW_WIDTH}
      opacity={FLOW_OPACITY}
      state={state()}
      label={state() === 'idle' ? null : { ...props.placed.label, text: f.weight }}
      testId="arch-port-link"
      class="arch-port-link"
      data={{ 'data-box': f.box, 'data-system': f.port }}
      onClick={() => set({ sel: { kind: 'port', box: f.box, system: f.port } })}
      title={`${props.names(f.box)} → ${props.names(f.port)}\n${t('arch.port.flowHint')}`}
    />
  )
}

const portsOf = (m: Model, data: Boxes): { ports: PortTarget[]; flows: PortFlow[] } => {
  const systems = systemsIn(m)
  if (!systems) return { ports: [], flows: [] }
  const weights = new Map<string, number>()
  for (const link of systems.portLinks) {
    const box = data.boxOfUnit.get(link.container)
    if (!box) continue
    const key = `${box}\u0000${link.system}`
    weights.set(key, (weights.get(key) ?? 0) + channelWeight(link))
  }
  const flows = [...weights].map(([key, weight]): PortFlow => {
    const [box, port] = key.split('\u0000')
    return { box, port, weight }
  })
  const used = new Set(flows.map((f) => f.port))
  const ports = systems.others
    .filter((s) => used.has(s.id))
    .map((s): PortTarget => ({ id: s.id, name: s.name, isExternal: s.isExternal }))
  return { ports, flows }
}

export const BoxesScene: Component<{ model: Model; data: Boxes; parent?: ArchGroup; fitKey: string }> = (props) => {
  const areaOf = (b: Box): string => {
    const layer = props.model.layers.find((l) => l.id === b.layer)?.name ?? '—'
    return b.kind === 'feature' ? t('arch.boxes.areaFeatures', { layer }) : layer
  }
  const ports = createMemo(() => portsOf(props.model, props.data))
  const layout = createMemo(() =>
    layoutBoxes(props.data, layerRanks(props.model.layers), ports().ports, ports().flows, orient(), areaOf),
  )
  const context = createMemo<string | null>((prev) => {
    const sel = nav().sel
    if (sel?.kind === 'box') return sel.key
    return sel?.kind === 'link' ? prev : null
  }, null)
  const focus = createMemo((): string | null => {
    const box = selectedBox()
    if (box !== null) return box
    const link = selectedLink()
    const last = context()
    return link && last && (link.from === last || link.to === last) ? last : null
  })
  const stubs = createMemo(() => shownStubs(layout().stubs, focus()))
  const shown = createMemo(() => {
    const base = shownLinks(layout().links, focus(), layout().stubbed)
    const picked = layout().links.find(isSelectedLink)
    const isDrawn = (l: BoxLink) => base.includes(l) || stubs().some((s) => s.link === l)
    return picked && !isDrawn(picked) ? [...base, picked] : base
  })
  const routed = createMemo(() => routeLinks(layout(), shown()))
  const hidden = () => Math.max(0, layout().links.length - shown().length - stubs().length)
  const names = (key: string) =>
    props.data.boxes.find((b) => b.key === key)?.name ?? ports().ports.find((p) => p.id === key)?.name ?? key
  const isDimmed = (key: string): boolean => {
    const sel = focus()
    if (sel === null || sel === key) return false
    return !shown().some((l) => l.from === key || l.to === key)
  }
  return (
    <Canvas
      bounds={layout().bounds}
      fitKey={`${props.fitKey}:${orient()}`}
      overlay={
        <>
          <OrientToggle />
          <Show when={hidden() > 0}>
            <BudgetNote text={t('arch.box.hidden', { n: hidden() })} />
          </Show>
        </>
      }
    >
      <g class="arch-boxes" data-orient={orient()}>
        <For each={layout().areas}>{(a) => <AreaMark area={a} />}</For>
        <g fill="none">
          <For each={routed().flows}>{(f) => <FlowMark placed={f} focus={focus()} names={names} />}</For>
          <For each={routed().links}>{(l) => <LinkMark placed={l} focus={focus()} names={names} />}</For>
        </g>
        <For each={stubs()}>{(s) => <StubMark stub={s} names={names} />}</For>
        <For each={layout().ports}>{(p) => <PortMark placed={p} />}</For>
        <For each={layout().boxes}>
          {(p) => (
            <BoxMark
              placed={p}
              isDimmed={isDimmed(p.box.key)}
              onOpen={() => openBox(props.model, p.box, props.parent)}
            />
          )}
        </For>
      </g>
    </Canvas>
  )
}
