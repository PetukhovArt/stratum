import { Accessor, Component, createMemo, createSignal, For, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { CANVAS_DEFS, EDGE_DASH, EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR } from '../../render/encoding'
import { Model } from '../../render/model'
import { go, nav, NavState, set } from '../../shell/nav'
import { layerTone, trunc } from '../focus/paint'
import { pairKey, SetAnalysis } from './analysis'
import {
  ARC_NODE,
  CurvedLink,
  CurveScene,
  layoutArc,
  layoutRing,
  layoutSetMatrix,
  RING_NODE,
  SET_CELL,
  SetCell,
  SetLink,
  SetMatrix,
  SetNode,
} from './layout'

const DIM_OPACITY = 0.1
const HIT_WIDTH = 10
const PATH_DASH = '4 3'
const CUT_DASH = '5 3'
const CHAIN_STROKE = 'oklch(90% 0.01 260)'
const BADGE = { w: 124, h: 20 } as const
const LABEL_CHARS = { ring: 24, arc: 19, matrix: 24 } as const
const COLUMN_LABEL_ANGLE = -60
const MATRIX_LABEL_GAP = 8

interface SceneProps {
  model: Model
  analysis: SetAnalysis
}

const selectedKey = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'edge' ? pairKey(sel.from, sel.to) : null
}

const pickEdge = (l: SetLink) => set({ sel: { kind: 'edge', from: l.from, to: l.to } })

const edgeColor = (l: SetLink): string => {
  if (l.worst) return SEV_COLOR[l.worst]
  return l.isChain ? CHAIN_STROKE : EDGE_NEUTRAL
}

const edgeTitle = (m: Model, l: SetLink): string =>
  `${m.modules.get(l.from)?.path}\n→ ${m.modules.get(l.to)?.path}${l.isPath ? `\n${t('set.edge.path')}` : ''}${
    l.rules ? `\n${l.rules}` : ''
  }`

const CutBadge: Component<{ link: CurvedLink; analysis: SetAnalysis }> = (props) => (
  <g class="set-badge" data-testid="set-cut" data-edge={props.link.key} transform={`translate(${props.link.badge.x},${props.link.badge.y})`}>
    <rect x={-BADGE.w / 2} y={-BADGE.h / 2} width={BADGE.w} height={BADGE.h} rx={BADGE.h / 2} />
    <text y="3.5" text-anchor="middle">
      {t('set.cutBadge', { freed: props.analysis.cuts[0].freed, n: props.analysis.loop?.length ?? 0 })}
    </text>
  </g>
)

const CurveLink: Component<{ model: Model; link: CurvedLink; hover: Accessor<number | null>; isArc: boolean }> = (props) => {
  const l = props.link
  const width = l.isCut && props.isArc ? 3 : l.isChain ? 2.2 : l.isPath ? 1 : 1.2
  const isLit = () => {
    const h = props.hover()
    return h === null || h === l.from || h === l.to
  }
  const dash = () => {
    if (l.isCut && props.isArc) return CUT_DASH
    return l.isPath ? PATH_DASH : EDGE_DASH[l.kind]
  }
  return (
    <g class="set-link" opacity={isLit() ? 1 : DIM_OPACITY} onClick={() => pickEdge(l)}>
      <path d={l.d} stroke="transparent" stroke-width={HIT_WIDTH} />
      <path
        d={l.d}
        data-edge={l.key}
        data-ch={l.worst ? 'sev' : 'edge'}
        data-rule={l.rules || undefined}
        stroke={edgeColor(l)}
        stroke-opacity={l.isChain || l.worst ? 0.95 : 0.4}
        stroke-width={selectedKey() === l.key ? width + 2 : width}
        stroke-dasharray={dash()}
        marker-end={CANVAS_DEFS.arrow}
      />
      <title>{edgeTitle(props.model, l)}</title>
    </g>
  )
}

const NodeMark: Component<{
  model: Model
  node: SetNode
  w: number
  h: number
  chars: number
  setHover: (id: number | null) => void
}> = (props) => {
  const m = props.model
  const n = props.node
  const layer = m.modules.get(n.id)!.layer
  const sev = m.worst(m.findingsOf(n.id))
  return (
    <g
      class={`set-node${n.isMember ? ' member' : ' via'}${n.isLoop ? ' loop' : ''}`}
      data-testid="set-node"
      data-id={n.id}
      transform={`translate(${n.x - props.w / 2},${n.y - props.h / 2})`}
      onMouseEnter={() => props.setHover(n.id)}
      onMouseLeave={() => props.setHover(null)}
      onClick={() => go({ view: 'focus', focus: n.id })}
    >
      <rect
        width={props.w}
        height={props.h}
        rx="6"
        stroke={n.isMember ? layerTone(m, layer, LAYER_TONE.tintEdgeStrong) : undefined}
      />
      <Show when={n.isMember}>
        <rect class="set-sw" width="4" height={props.h} rx="2" fill={layerTone(m, layer)} />
      </Show>
      <text x="12" y={props.h / 2 + 3.5}>
        {trunc(m.label(n.id), props.chars)}
      </text>
      <Show when={sev}>
        {(s) => <circle data-ch="sev" cx={props.w - 10} cy={props.h / 2} r="3" fill={SEV_COLOR[s()]} />}
      </Show>
      <title>{m.modules.get(n.id)!.path + (n.isMember ? '' : `\n${t('set.node.via')}`)}</title>
    </g>
  )
}

const CurveMarks: Component<{ model: Model; analysis: SetAnalysis; scene: CurveScene; isArc: boolean }> = (props) => {
  const [hover, setHover] = createSignal<number | null>(null)
  const size = (n: SetNode) => {
    if (props.isArc) return { w: ARC_NODE.w, h: ARC_NODE.h, chars: LABEL_CHARS.arc }
    return { w: RING_NODE.w, h: n.isMember ? RING_NODE.h : RING_NODE.viaH, chars: LABEL_CHARS.ring }
  }
  const cut = () => props.scene.links.find((l) => l.isCut)
  return (
    <g class={`set-scene${props.isArc ? ' arc' : ' ring'}`}>
      <Show when={props.isArc && props.scene.nodes.length}>
        <line class="set-axis" x1={-ARC_NODE.w / 2} y1={0} x2={(props.scene.nodes.length - 1) * ARC_NODE.spacing + ARC_NODE.w / 2} y2={0} />
      </Show>
      <g fill="none">
        <For each={props.scene.links}>{(l) => <CurveLink model={props.model} link={l} hover={hover} isArc={props.isArc} />}</For>
      </g>
      <For each={props.scene.nodes}>
        {(n) => <NodeMark model={props.model} node={n} {...size(n)} setHover={setHover} />}
      </For>
      <Show when={cut()}>{(l) => <CutBadge link={l()} analysis={props.analysis} />}</Show>
    </g>
  )
}

const MatrixMarks: Component<{ model: Model; matrix: SetMatrix }> = (props) => {
  const mx = () => props.matrix
  const span = () => mx().order.length * SET_CELL
  const cellFill = (l: SetCell) => {
    if (l.worst) return SEV_COLOR[l.worst]
    return l.isAbove ? 'oklch(78% 0.02 260)' : 'oklch(60% 0.015 260)'
  }
  return (
    <g class="set-mx">
      <rect class="set-mx-frame" width={span()} height={span()} />
      <line class="set-mx-diag" x1={0} y1={0} x2={span()} y2={span()} />
      <For each={mx().cells}>
        {(c) => (
          <rect
            class={`set-mx-cell${c.isCut ? ' cut' : ''}`}
            data-testid={c.isCut ? 'set-cut' : undefined}
            data-edge={c.key}
            data-ch={c.worst ? 'sev' : 'cell'}
            data-rule={c.rules || undefined}
            x={c.col * SET_CELL + 2}
            y={c.row * SET_CELL + 2}
            width={SET_CELL - 4}
            height={SET_CELL - 4}
            rx="3"
            fill={cellFill(c)}
            stroke-width={selectedKey() === c.key ? 3 : 2}
            onClick={() => pickEdge(c)}
          >
            <title>{edgeTitle(props.model, c)}</title>
          </rect>
        )}
      </For>
      <Show when={mx().block}>
        {(b) => (
          <rect
            class="set-mx-block"
            x={b().start * SET_CELL}
            y={b().start * SET_CELL}
            width={(b().end - b().start + 1) * SET_CELL}
            height={(b().end - b().start + 1) * SET_CELL}
          />
        )}
      </Show>
      <For each={mx().order}>
        {(id, i) => {
          const name = trunc(props.model.label(id), LABEL_CHARS.matrix)
          const isMember = mx().members.has(id)
          return (
            <g class={`set-mx-label${isMember ? ' member' : ''}`} onClick={() => go({ view: 'focus', focus: id })}>
              <text x={-MATRIX_LABEL_GAP} y={i() * SET_CELL + SET_CELL / 2 + 3.5} text-anchor="end">
                {name}
              </text>
              <text
                transform={`translate(${i() * SET_CELL + SET_CELL / 2 + 3},${-MATRIX_LABEL_GAP}) rotate(${COLUMN_LABEL_ANGLE})`}
              >
                {name}
              </text>
              <title>{props.model.modules.get(id)?.path}</title>
            </g>
          )
        }}
      </For>
    </g>
  )
}

const Legend: Component<{ variant: 'ring' | 'arc' | 'matrix' }> = (props) => (
  <div class="legend set-legend">
    <Show
      when={props.variant !== 'matrix'}
      fallback={
        <>
          <div class="legend-row">
            <span class="mono set-lg">r→c</span>
            {t('set.legend.rowCol')}
          </div>
          <div class="legend-row">
            <span class="set-lg set-lg-cut" />
            {t('set.legend.cut')} · {t('set.legend.block')}
          </div>
        </>
      }
    >
      <div class="legend-row">
        <span class="set-lg set-lg-member" />
        {t('set.legend.member')}
        <span class="set-lg set-lg-via" />
        {t('set.legend.via')}
      </div>
      <div class="legend-row">
        <span class="set-lg set-lg-chain" />
        {t('set.legend.chain')}
      </div>
      <div class="legend-row">
        <span class="set-lg set-lg-path" />
        {t('set.legend.path')}
      </div>
      <Show when={props.variant === 'arc'}>
        <div class="legend-row">
          <span class="set-lg" />
          {t('set.legend.back')}
        </div>
      </Show>
    </Show>
  </div>
)

type Laid = { kind: 'curves'; scene: CurveScene } | { kind: 'matrix'; matrix: SetMatrix }

const layoutOf = (m: Model, a: SetAnalysis, variant: NavState['setVariant']): Laid => {
  switch (variant) {
    case 'ring':
      return { kind: 'curves', scene: layoutRing(m, a) }
    case 'arc':
      return { kind: 'curves', scene: layoutArc(m, a) }
    case 'matrix':
      return { kind: 'matrix', matrix: layoutSetMatrix(m, a) }
  }
}

export const SetScene: Component<SceneProps> = (props) => {
  const variant = () => nav().setVariant
  const laid = createMemo(() => layoutOf(props.model, props.analysis, variant()))
  const bounds = () => {
    const l = laid()
    return l.kind === 'curves' ? l.scene.bounds : l.matrix.bounds
  }
  const fitKey = createMemo(() => `${variant()}:${props.analysis.members.join(',')}`)
  return (
    <Canvas bounds={bounds()} fitKey={fitKey()} overlay={<Legend variant={variant()} />}>
      {(() => {
        const l = laid()
        return l.kind === 'curves' ? (
          <CurveMarks model={props.model} analysis={props.analysis} scene={l.scene} isArc={variant() === 'arc'} />
        ) : (
          <MatrixMarks model={props.model} matrix={l.matrix} />
        )
      })()}
    </Canvas>
  )
}
