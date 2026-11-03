import { Component, createMemo, For, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { sharedNote } from '../../features/shared-mark'
import { Edge, EdgeState } from '../../render/edge'
import { EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR, SHARED_COLOR } from '../../render/encoding'
import { edgeKey, Model } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { EdgeMark, GROUP_CHARS, GroupBox, groupMeta, layoutRegion, RegionLayout, stageText } from './layout'
import { ellipsizePath, trunc } from '../../render/labels'
import { ADDED_STROKE, labelOf, layerTint, REMOVED_DASH, rulesOf } from './paint'
import { Review, ReviewNode, worstBreak } from './review'

const LABEL_CHARS = 24
const STAGE_STROKE = 'oklch(92% 0.01 260)'
const BADGE_BASELINE = 4.5
const LEGEND_HALO = 'oklch(18% 0.012 260)'
const OPACITY = { removed: 0.6, changed: 0.95, same: 0.3 } as const
const WIDTH = { changed: 2, same: 1.1 } as const

const selectedKey = (): string | null => {
  const sel = nav().sel
  return sel?.kind === 'edge' ? edgeKey(sel.from, sel.to) : null
}

const ChangeEdge: Component<{ mark: EdgeMark; review: Review }> = (props) => {
  const e = props.mark.edge
  const sev = worstBreak(e.breaks)
  const isChanged = e.tag !== 'same'
  const color = sev ? SEV_COLOR[sev] : e.tag === 'added' ? ADDED_STROKE : EDGE_NEUTRAL
  const isSelected = () => selectedKey() === e.key
  const state = (): EdgeState => (isSelected() ? 'selected' : selectedKey() === null ? 'idle' : 'dimmed')
  const title = () => {
    const from = props.review.nodes.get(e.from)?.path ?? ''
    const to = props.review.nodes.get(e.to)?.path ?? ''
    const rules = e.breaks.length ? `\n${rulesOf(e.breaks)}` : ''
    return `${from}\n→ ${to}\n${t(`mr.tag.${e.tag}`)}${rules}`
  }
  return (
    <Edge
      d={props.mark.d}
      kind={sev ? 'break' : 'neutral'}
      severity={sev}
      imports={e.kind}
      dash={e.tag === 'removed' ? REMOVED_DASH : undefined}
      color={color}
      width={isChanged ? WIDTH.changed : WIDTH.same}
      opacity={e.tag === 'removed' ? OPACITY.removed : isChanged || sev ? OPACITY.changed : OPACITY.same}
      state={state()}
      testId="mr-edge"
      class="mr-edge"
      pathData={{ 'data-edge': e.key, 'data-tag': e.tag, 'data-rule': sev ? rulesOf(e.breaks) : undefined }}
      label={
        isChanged
          ? {
              x: props.mark.mid.x,
              y: props.mark.mid.y + BADGE_BASELINE,
              text: e.tag === 'added' ? '+' : '−',
              class: 'mr-badge',
              color,
            }
          : null
      }
      hit={false}
      onClick={() => set({ sel: isSelected() ? null : { kind: 'edge', from: e.from, to: e.to } })}
      title={title()}
    />
  )
}

const Group: Component<{ box: GroupBox; model: Model; review: Review }> = (props) => {
  const b = props.box
  const change = b.container === null ? undefined : props.review.stageOf.get(b.container)
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
        {groupMeta(props.model, props.review, b)}
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

const LegendLine: Component<{ color: string; dash?: string; glyph: string }> = (props) => (
  <svg width="26" height="16">
    <line x1="0" y1="8" x2="26" y2="8" stroke={props.color} stroke-width={WIDTH.changed} stroke-dasharray={props.dash} />
    <text class="mr-badge" x="13" y={8 + BADGE_BASELINE} fill={props.color} stroke={LEGEND_HALO}>
      {props.glyph}
    </text>
  </svg>
)

const Legend: Component = () => (
  <div class="legend mr-legend">
    <div class="legend-row">
      <LegendLine color={ADDED_STROKE} glyph="+" />
      {t('mr.legend.added')}
    </div>
    <div class="legend-row">
      <LegendLine color={EDGE_NEUTRAL} dash={REMOVED_DASH} glyph="−" />
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
        <For each={layout().edges}>{(mark) => <ChangeEdge mark={mark} review={props.review} />}</For>
      </g>
      <For each={[...props.review.nodes.values()]}>
        {(n) => <Node node={n} layout={layout()} model={props.model} />}
      </For>
    </Canvas>
  )
}
