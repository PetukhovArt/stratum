import { Component, createMemo, For, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { sharedNote } from '../../features/shared-mark'
import { CANVAS_DEFS, EDGE_DASH, LAYER_TONE, SHARED_COLOR } from '../../render/encoding'
import { edgeKey, Model } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { EdgeMark, GroupBox, layoutRegion, RegionLayout } from './layout'
import { ellipsizePath, trunc } from '../../render/labels'
import { ADDED_STROKE, breakColor, labelOf, layerTint, NEUTRAL_STROKE, REMOVED_DASH, rulesOf } from './paint'
import { Review, ReviewNode } from './review'

const DIM_OPACITY = 0.15
const HIT_WIDTH = 9
const BADGE_R = 8
const LABEL_CHARS = 24
const GROUP_CHARS = 26
const STAGE_STROKE = 'oklch(92% 0.01 260)'

const selectedKey = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'edge' ? edgeKey(sel.from, sel.to) : null
}

const stageText = (review: Review, container: number | null, fallback: number | undefined): string => {
  const change = container === null ? undefined : review.stageOf.get(container)
  if (change) return change.base === null ? `→ s${change.head}` : `s${change.base} → s${change.head}`
  return fallback ? `s${fallback}` : ''
}

const Edge: Component<{ mark: EdgeMark; review: Review }> = (props) => {
  const e = props.mark.edge
  const sev = breakColor(e.breaks)
  const isChanged = e.tag !== 'same'
  const color = sev ?? (e.tag === 'added' ? ADDED_STROKE : NEUTRAL_STROKE)
  const opacity = e.tag === 'removed' ? 0.6 : isChanged || sev ? 0.95 : 0.3
  const isSelected = () => selectedKey() === e.key
  const isDim = () => selectedKey() !== null && !isSelected()
  const width = () => (isChanged ? 2 : 1.1) + (isSelected() ? 1.5 : 0)
  const title = () => {
    const from = props.review.nodes.get(e.from)?.path ?? ''
    const to = props.review.nodes.get(e.to)?.path ?? ''
    const rules = e.breaks.length ? `\n${rulesOf(e.breaks)}` : ''
    return `${from}\n→ ${to}\n${t(`mr.tag.${e.tag}`)}${rules}`
  }
  return (
    <g
      class="mr-edge"
      opacity={isDim() ? DIM_OPACITY : 1}
      onClick={() => set({ sel: isSelected() ? null : { kind: 'edge', from: e.from, to: e.to } })}
    >
      <path d={props.mark.d} stroke="transparent" stroke-width={HIT_WIDTH} fill="none" />
      <path
        d={props.mark.d}
        fill="none"
        data-edge={e.key}
        data-tag={e.tag}
        data-ch={sev ? 'sev' : 'edge'}
        data-rule={sev ? rulesOf(e.breaks) : undefined}
        stroke={color}
        stroke-opacity={opacity}
        stroke-width={width()}
        stroke-dasharray={e.tag === 'removed' ? REMOVED_DASH : EDGE_DASH[e.kind]}
        marker-end={CANVAS_DEFS.arrow}
      />
      <Show when={isChanged}>
        <g data-ch={sev ? 'sev' : 'diff'} transform={`translate(${props.mark.mid.x},${props.mark.mid.y})`}>
          <circle r={BADGE_R} fill="oklch(18% 0.012 260)" stroke={color} />
          <text y="3.5" font-size="11" font-weight="700" text-anchor="middle" fill={color}>
            {e.tag === 'added' ? '+' : '−'}
          </text>
        </g>
      </Show>
      <title>{title()}</title>
    </g>
  )
}

const Group: Component<{ box: GroupBox; model: Model; review: Review }> = (props) => {
  const b = props.box
  const change = b.container === null ? undefined : props.review.stageOf.get(b.container)
  const stage = b.container === null ? undefined : props.model.containers.get(b.container)?.stage
  return (
    <g>
      <rect
        x={b.x}
        y={b.y}
        width={b.w}
        height={b.h}
        rx="6"
        fill={layerTint(props.model, b.layer, LAYER_TONE.tint)}
        stroke={change ? STAGE_STROKE : layerTint(props.model, b.layer, LAYER_TONE.tintEdge)}
        stroke-width={change ? 1.6 : 1}
        stroke-dasharray={b.isPrivate ? '4 3' : undefined}
      />
      <text x={b.x + 9} y={b.y + 16} font-size="10" fill="oklch(80% 0.006 260)">
        {trunc(b.name, GROUP_CHARS)}
      </text>
      <text
        x={b.x + b.w - 9}
        y={b.y + 16}
        font-size="9.5"
        text-anchor="end"
        fill={change ? STAGE_STROKE : 'oklch(58% 0.012 260)'}
      >
        {b.isPrivate ? 'private' : stageText(props.review, b.container, stage)}
      </text>
    </g>
  )
}

const Node: Component<{ node: ReviewNode; layout: RegionLayout; model: Model }> = (props) => {
  const n = props.node
  const r = props.layout.nodes.get(n.id)!
  const isShared = !n.isGhost && props.model.isSharedByChosenFeatures(n.id)
  return (
    <g
      class={n.isGhost ? 'mr-node ghost' : 'mr-node'}
      data-testid="mr-node"
      data-ghost={n.isGhost ? '1' : undefined}
      data-shared={isShared ? 'true' : undefined}
      transform={`translate(${r.x},${r.y})`}
      onClick={() => {
        if (!n.isGhost) go({ view: 'focus', focus: n.id })
      }}
    >
      <rect
        width={r.w}
        height={r.h}
        rx="4"
        fill={n.isGhost ? 'transparent' : 'oklch(23% 0.016 260)'}
        stroke={isShared ? SHARED_COLOR : n.isImporter ? 'oklch(88% 0.01 260)' : 'oklch(42% 0.012 260)'}
        stroke-dasharray={n.isGhost ? '3 3' : undefined}
      />
      <text x="8" y="14.5" font-size="10" fill={n.isGhost ? 'oklch(68% 0.01 260)' : 'oklch(90% 0.005 260)'}>
        {ellipsizePath(labelOf(props.model, n.id, n.path), LABEL_CHARS)}
      </text>
      <Show when={n.isImporter}>
        <text x={r.w - 8} y="14.5" font-size="9" text-anchor="end" font-weight="700" fill="oklch(88% 0.01 260)">
          M
        </text>
      </Show>
      <title>{n.path + (isShared ? sharedNote(props.model, n.id) : '')}</title>
    </g>
  )
}

const Legend: Component = () => (
  <div class="legend mr-legend">
    <div class="legend-row">
      <svg width="26" height="16">
        <line x1="0" y1="8" x2="26" y2="8" stroke={ADDED_STROKE} stroke-width="2" />
        <circle cx="13" cy="8" r="6" fill="oklch(18% 0.012 260)" stroke={ADDED_STROKE} />
        <text x="13" y="11" font-size="9" text-anchor="middle" fill={ADDED_STROKE}>
          +
        </text>
      </svg>
      {t('mr.legend.added')}
    </div>
    <div class="legend-row">
      <svg width="26" height="16">
        <line x1="0" y1="8" x2="26" y2="8" stroke={NEUTRAL_STROKE} stroke-width="2" stroke-dasharray={REMOVED_DASH} />
        <circle cx="13" cy="8" r="6" fill="oklch(18% 0.012 260)" stroke={NEUTRAL_STROKE} />
        <text x="13" y="11" font-size="9" text-anchor="middle" fill={NEUTRAL_STROKE}>
          −
        </text>
      </svg>
      {t('mr.legend.removed')}
    </div>
    <div class="legend-row">
      <span class="mr-lg-sev" data-ch="sev" />
      {t('mr.legend.sev')}
    </div>
    <div class="legend-row">{t('mr.legend.importer')}</div>
    <div class="legend-row">{t('mr.legend.ghost')}</div>
  </div>
)

export const RegionScene: Component<{ model: Model; review: Review }> = (props) => {
  const layout = createMemo(() => layoutRegion(props.model, props.review))
  const layerName = (layer: number | null) =>
    layer === null ? t('mr.lane.baseOnly') : (props.model.layers.find((l) => l.id === layer)?.name ?? '')
  return (
    <Canvas bounds={layout().bounds} fitKey={layout()} legend={<Legend />}>
      <For each={layout().lanes}>
        {(l) => (
          <g>
            <rect
              x={l.x}
              y={l.y}
              width={l.w}
              height={l.h}
              rx="10"
              fill={layerTint(props.model, l.layer, LAYER_TONE.band)}
              stroke={layerTint(props.model, l.layer, LAYER_TONE.tintEdge)}
            />
            <rect x={l.x} y={l.y} width="4" height={l.h} rx="2" fill={layerTint(props.model, l.layer)} />
            <text
              x={l.x + 16}
              y={l.y + 17}
              font-size="10.5"
              font-weight="600"
              letter-spacing="1"
              fill={layerTint(props.model, l.layer, LAYER_TONE.label)}
            >
              {layerName(l.layer).toUpperCase()}
            </text>
          </g>
        )}
      </For>
      <For each={layout().slices}>
        {(s) => {
          const c = props.model.containers.get(s.container)
          const change = props.review.stageOf.get(s.container)
          return (
            <g>
              <rect
                x={s.x}
                y={s.y}
                width={s.w}
                height={s.h}
                rx="8"
                fill="oklch(20% 0.012 260 / .6)"
                stroke={change ? STAGE_STROKE : 'oklch(45% 0.012 260 / .7)'}
                stroke-width={change ? 1.6 : 1}
              />
              <text x={s.x + 10} y={s.y + 15} font-size="10.5" font-weight="600" fill="oklch(88% 0.006 260)">
                {c?.name ?? ''}
              </text>
              <text
                x={s.x + s.w - 10}
                y={s.y + 15}
                font-size="9.5"
                text-anchor="end"
                fill={change ? STAGE_STROKE : 'oklch(58% 0.012 260)'}
              >
                {stageText(props.review, s.container, c?.stage)}
              </text>
            </g>
          )
        }}
      </For>
      <For each={layout().groups}>{(b) => <Group box={b} model={props.model} review={props.review} />}</For>
      <g data-testid="mr-edges">
        <For each={layout().edges}>{(mark) => <Edge mark={mark} review={props.review} />}</For>
      </g>
      <For each={[...props.review.nodes.values()]}>
        {(n) => <Node node={n} layout={layout()} model={props.model} />}
      </For>
    </Canvas>
  )
}
