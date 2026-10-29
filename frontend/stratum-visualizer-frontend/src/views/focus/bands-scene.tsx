import { Component, createMemo, For, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { sharedNote } from '../../features/shared-mark'
import { t } from '../../i18n'
import { CANVAS_DEFS, EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR, SHARED_COLOR } from '../../render/encoding'
import { Model, worst } from '../../render/model'
import { go, set } from '../../shell/nav'
import { Band, BAND, BAND_CARD_SCALE, BandLink, BandsLayout, layoutBands, Pill } from './bands'
import { FocusCard } from './focus-card'
import { trunc } from '../../render/labels'
import { layerTone, rulesOf, curve } from './paint'

const BAR_W = 6
const LABEL_INSET = 12
const DIM_TEXT = 'oklch(52% 0.012 260)'
const DASHED = '3 2'

const rightLabel = (b: Band): string => {
  if (!b.isImportAllowed) return `${b.layer.name} · ${t('focus.band.mayNotImport')}`
  return b.imports.length ? b.layer.name : `${b.layer.name} · ${t('focus.band.allowedUnused')}`
}

const leftLabel = (b: Band): string =>
  b.isImportedByAllowed ? b.layer.name : `${b.layer.name} · ${t('focus.band.mayNotDepend')}`

const BandRow: Component<{ model: Model; band: Band }> = (props) => {
  const b = props.band
  const m = props.model
  const tone = (isAllowed: boolean) => layerTone(m, b.layer.id, isAllowed ? LAYER_TONE.band : LAYER_TONE.bandOff)
  return (
    <g data-testid="focus-band" data-layer={b.layer.id}>
      <rect
        data-testid="band-left"
        data-allowed={String(b.isImportedByAllowed)}
        x={-BAND.half}
        y={b.y}
        width={BAND.half}
        height={b.h}
        fill={tone(b.isImportedByAllowed)}
      />
      <rect
        data-testid="band-right"
        data-allowed={String(b.isImportAllowed)}
        x="0"
        y={b.y}
        width={BAND.half}
        height={b.h}
        fill={tone(b.isImportAllowed)}
      />
      <rect x={-BAR_W / 2} y={b.y} width={BAR_W} height={b.h} fill={layerTone(m, b.layer.id)} />
      <text
        x={LABEL_INSET}
        y={b.y + b.h - 8}
        font-size="9.5"
        fill={b.isImportAllowed ? layerTone(m, b.layer.id, LAYER_TONE.label) : DIM_TEXT}
      >
        {rightLabel(b)}
      </text>
      <text
        x={-LABEL_INSET}
        y={b.y + b.h - 8}
        font-size="9.5"
        text-anchor="end"
        fill={b.isImportedByAllowed ? layerTone(m, b.layer.id, LAYER_TONE.label) : DIM_TEXT}
      >
        {leftLabel(b)}
      </text>
    </g>
  )
}

const Link: Component<{ link: BandLink }> = (props) => {
  const l = props.link
  const sev = worst(l.pill.findings)
  return (
    <path
      data-edge={l.key}
      data-ch={sev ? 'sev' : 'edge'}
      data-rule={sev ? rulesOf(l.pill.findings) : undefined}
      d={curve(l.x1, l.y1, l.x2, l.y2)}
      fill="none"
      stroke={sev ? SEV_COLOR[sev] : EDGE_NEUTRAL}
      stroke-opacity={sev ? 0.9 : 0.4}
      stroke-width={1 + Math.min(3, Math.log2(l.pill.ids.length + 1))}
      marker-end={CANVAS_DEFS.arrow}
    />
  )
}

const PillView: Component<{ model: Model; pill: Pill }> = (props) => {
  const p = props.pill
  const m = props.model
  const sev = worst(p.findings)
  const isSingle = p.ids.length === 1
  const isShared = isSingle && m.isSharedByChosenFeatures(p.ids[0])
  const name = isSingle ? m.label(p.ids[0]) : p.group.name
  const onClick = () => (isSingle ? go({ view: 'focus', focus: p.ids[0] }) : set({ sel: { kind: 'imports', pairs: p.pairs } }))
  const isRight = p.side > 0
  return (
    <g
      class="fo-pill"
      data-testid="focus-pill"
      data-shared={isShared ? 'true' : undefined}
      onClick={onClick}
    >
      <rect
        x={p.x}
        y={p.y}
        width={p.w}
        height={p.h}
        rx="11"
        fill={layerTone(m, p.group.layer, LAYER_TONE.pill)}
        stroke={isShared ? SHARED_COLOR : layerTone(m, p.group.layer, LAYER_TONE.tintEdgeStrong)}
        stroke-dasharray={sev ? DASHED : undefined}
      />
      <Show when={sev}>
        <rect x={p.x} y={p.y} width={p.w} height={p.h} rx="11" fill={CANVAS_DEFS.hatch} pointer-events="none" />
        <circle data-ch="sev" cx={isRight ? p.x + p.w - 10 : p.x + 10} cy={p.y + 11} r="3.5" fill={SEV_COLOR[sev!]} />
      </Show>
      <text
        x={isRight ? p.x + 10 : p.x + p.w - 10}
        y={p.y + 15}
        text-anchor={isRight ? 'start' : 'end'}
        font-size="10"
        fill="oklch(88% 0.006 260)"
        text-decoration={sev ? 'line-through' : undefined}
      >
        {trunc(name, 21)}
        {isSingle ? '' : ` · ${p.ids.length}`}
      </text>
      <title>{p.group.path + (isShared ? sharedNote(m, p.ids[0]) : '')}</title>
    </g>
  )
}

const pillsOf = (layout: BandsLayout): Pill[] => layout.bands.flatMap((b) => [...b.imports, ...b.importers])

export const BandsScene: Component<{ model: Model; focus: number }> = (props) => {
  const layout = createMemo(() => layoutBands(props.model, props.focus))
  return (
    <Canvas bounds={layout().bounds} fitKey={`bands:${props.focus}`}>
      <g data-testid="focus-bands">
        <For each={layout().bands}>{(b) => <BandRow model={props.model} band={b} />}</For>
        <For each={layout().links}>{(l) => <Link link={l} />}</For>
        <For each={pillsOf(layout())}>{(p) => <PillView model={props.model} pill={p} />}</For>
        <g transform={`translate(0,${layout().focusY}) scale(${BAND_CARD_SCALE})`}>
          <FocusCard model={props.model} id={props.focus} />
        </g>
      </g>
    </Canvas>
  )
}
