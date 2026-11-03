import { Component, createMemo, For, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { sharedNote } from '../../features/shared-mark'
import { Edge, EdgeState } from '../../render/edge'
import { EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR, SHARED_COLOR } from '../../render/encoding'
import { Model } from '../../render/model'
import { go, nav, Selection, set } from '../../shell/nav'
import { EdgeMark, FoldMark, GroupBox, groupHead, layoutRegion, RegionLayout, routeEdges, stageText } from './layout'
import { ellipsizePath, trunc } from '../../render/labels'
import { ADDED_STROKE, labelOf, layerTint, REMOVED_DASH, rulesOf } from './paint'
import { edgeOf, Fold, linesOfModule, Review, ReviewEdge, ReviewNode, worstBreak } from './review'

const LABEL_CHARS = 24
const STAGE_STROKE = 'oklch(92% 0.01 260)'
const BADGE_BASELINE = 4.5
const FOLD_BASELINE = 4
const LEGEND_HALO = 'oklch(18% 0.012 260)'
const LABEL_HALO = 'oklch(16% 0.012 260)'
const LABEL_DIMMED = 0.3
const LEGEND_SPAN = { badge: 26, fold: 58 } as const
const OPACITY = { removed: 0.6, changed: 0.95, same: 0.6, fold: 0.85 } as const
const WIDTH = { changed: 2, same: 1.4 } as const
const NODE_DIMMED = 0.3
const SELECTED_STROKE = 'oklch(97% 0.005 260)'
const FOLD_WIDTH = { base: 2, perDoubling: 1, max: 8 } as const

const foldWidth = (f: Fold): number =>
  Math.min(FOLD_WIDTH.max, FOLD_WIDTH.base + Math.log2(f.added + f.removed + 1) * FOLD_WIDTH.perDoubling)

interface RegionFocus {
  lines: ReviewEdge[]
  litLines: ReadonlySet<string> | null
  litNodes: ReadonlySet<number> | null
  fold: string | null
  edge: string | null
  module: number | null
}

const IDLE: Omit<RegionFocus, 'lines'> = { litLines: null, litNodes: null, fold: null, edge: null, module: null }

const lit = (edges: readonly ReviewEdge[], also: readonly number[] = []) => ({
  litLines: new Set(edges.map((e) => e.key)),
  litNodes: new Set([...also, ...edges.flatMap((e) => [e.from, e.to])]),
})

const withAlways = (review: Review, extra: readonly ReviewEdge[]): ReviewEdge[] => {
  const byKey = new Map(review.alwaysDrawn.map((e) => [e.key, e]))
  for (const e of extra) byKey.set(e.key, e)
  return [...byKey.values()]
}

const focusOf = (m: Model, review: Review, sel: Selection | null): RegionFocus => {
  const idle = { ...IDLE, lines: [...review.alwaysDrawn] }
  if (sel?.kind === 'module' && review.nodes.has(sel.id)) {
    const own = linesOfModule(m, review, sel.id)
    return { ...idle, ...lit(own, [sel.id]), lines: withAlways(review, own), module: sel.id }
  }
  if (sel?.kind === 'link') {
    const fold = review.folds.find((f) => f.from === sel.from && f.to === sel.to)
    if (!fold) return idle
    return { ...idle, ...lit(fold.edges), lines: withAlways(review, fold.edges), fold: fold.key }
  }
  if (sel?.kind === 'edge') {
    const edge = edgeOf(m, review, sel.from, sel.to)
    if (!edge) return idle
    return { ...idle, ...lit([edge]), lines: withAlways(review, [edge]), edge: edge.key }
  }
  return idle
}

const lineState = (focus: RegionFocus, key: string): EdgeState => {
  if (focus.edge === key) return 'selected'
  if (!focus.litLines || focus.litLines.has(key)) return 'idle'
  return 'dimmed'
}

const edgeTitle = (review: Review, e: ReviewEdge): string => {
  const from = review.nodes.get(e.from)?.path ?? ''
  const to = review.nodes.get(e.to)?.path ?? ''
  const rules = e.breaks.length ? `\n${rulesOf(e.breaks)}` : ''
  return `${from}\n→ ${to}\n${t(`mr.tag.${e.tag}`)}${rules}`
}

const ChangeEdge: Component<{ mark: EdgeMark; review: Review; focus: RegionFocus }> = (props) => {
  const e = props.mark.edge
  const sev = worstBreak(e.breaks)
  const isChanged = e.tag !== 'same'
  const color = sev ? SEV_COLOR[sev] : e.tag === 'added' ? ADDED_STROKE : EDGE_NEUTRAL
  const state = () => lineState(props.focus, e.key)
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
      onClick={() => set({ sel: state() === 'selected' ? null : { kind: 'edge', from: e.from, to: e.to } })}
      title={edgeTitle(props.review, e)}
    />
  )
}

const foldText = (f: Fold): string => `+${f.added} −${f.removed}`

const foldColor = (f: Fold): string => (f.worst ? SEV_COLOR[f.worst] : f.added ? ADDED_STROKE : EDGE_NEUTRAL)

const foldState = (focus: RegionFocus, f: Fold): EdgeState =>
  focus.fold === f.key ? 'selected' : focus.litLines ? 'dimmed' : 'idle'

const toggleFold = (focus: RegionFocus, f: Fold): void =>
  set({ sel: focus.fold === f.key ? null : { kind: 'link', from: f.from, to: f.to } })

const FoldEdge: Component<{ mark: FoldMark; review: Review; focus: RegionFocus }> = (props) => {
  const f = props.mark.fold
  const pathOf = (key: string) => [...props.review.nodes.values()].find((n) => n.group.key === key)?.group.path ?? key
  return (
    <Edge
      d={props.mark.d}
      kind={f.worst ? 'break' : 'neutral'}
      severity={f.worst}
      dash={f.added ? undefined : REMOVED_DASH}
      color={foldColor(f)}
      width={foldWidth(f)}
      opacity={OPACITY.fold}
      state={foldState(props.focus, f)}
      testId="mr-fold"
      class="mr-fold"
      data={{ 'data-fold': f.key }}
      pathData={{ 'data-edge': `fold:${f.key}`, 'data-tag': 'fold', 'data-rule': f.worst ?? undefined }}
      onClick={() => toggleFold(props.focus, f)}
      title={`${pathOf(f.from)}\n→ ${pathOf(f.to)}\n${foldText(f)}`}
    />
  )
}

const FoldLabel: Component<{ mark: FoldMark; focus: RegionFocus }> = (props) => {
  const f = props.mark.fold
  return (
    <text
      class="mr-fold-label"
      data-testid="mr-fold-label"
      data-fold={f.key}
      data-ch={f.worst ? 'sev' : 'edge'}
      x={props.mark.mid.x}
      y={props.mark.mid.y + FOLD_BASELINE}
      fill={foldColor(f)}
      stroke={LABEL_HALO}
      opacity={foldState(props.focus, f) === 'dimmed' ? LABEL_DIMMED : 1}
      onClick={() => toggleFold(props.focus, f)}
    >
      {foldText(f)}
    </text>
  )
}

const Group: Component<{ box: GroupBox; model: Model; review: Review }> = (props) => {
  const b = props.box
  const change = b.container === null ? undefined : props.review.stageOf.get(b.container)
  const head = groupHead(props.model, props.review, b)
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
        {trunc(b.name, head.nameChars)}
      </text>
      <Show when={head.inner}>
        <text
          x={head.innerRight}
          y={b.y + 16}
          font-size="9.5"
          font-weight="600"
          text-anchor="end"
          fill="oklch(88% 0.008 260)"
          data-testid="mr-group-inner"
          data-group={b.key}
        >
          {head.inner}
        </text>
      </Show>
      <text
        x={b.x + b.w - 9}
        y={b.y + 16}
        font-size="9.5"
        text-anchor="end"
        fill={change ? STAGE_STROKE : 'oklch(58% 0.012 260)'}
      >
        {head.meta}
      </text>
    </g>
  )
}

const Node: Component<{ node: ReviewNode; layout: RegionLayout; model: Model; focus: RegionFocus }> = (props) => {
  const n = props.node
  const r = props.layout.nodes.get(n.id)!
  const isShared = !n.isGhost && props.model.isSharedByChosenFeatures(n.id)
  const isSelected = () => props.focus.module === n.id
  const isDimmed = () => props.focus.litNodes !== null && !props.focus.litNodes.has(n.id)
  const stroke = () => {
    if (isSelected()) return SELECTED_STROKE
    if (isShared) return SHARED_COLOR
    return n.isImporter ? 'oklch(88% 0.01 260)' : 'oklch(42% 0.012 260)'
  }
  return (
    <g
      class={n.isGhost ? 'mr-node ghost' : 'mr-node'}
      data-testid="mr-node"
      data-id={n.id}
      data-ghost={n.isGhost ? '1' : undefined}
      data-shared={isShared ? 'true' : undefined}
      data-state={isSelected() ? 'selected' : isDimmed() ? 'dimmed' : undefined}
      transform={`translate(${r.x},${r.y})`}
      opacity={isDimmed() ? NODE_DIMMED : undefined}
      onClick={() => set({ sel: { kind: 'module', id: n.id } })}
      onDblClick={() => {
        if (!n.isGhost) go({ view: 'focus', focus: n.id })
      }}
    >
      <rect
        width={r.w}
        height={r.h}
        rx="4"
        fill={n.isGhost ? 'transparent' : 'oklch(23% 0.016 260)'}
        stroke={stroke()}
        stroke-width={isSelected() ? 2.4 : 1}
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

const LegendLine: Component<{ color: string; dash?: string; glyph: string; isFold?: boolean }> = (props) => {
  const span = props.isFold ? LEGEND_SPAN.fold : LEGEND_SPAN.badge
  return (
    <svg width={span} height="16">
      <line
        x1="0"
        y1="8"
        x2={span}
        y2="8"
        stroke={props.color}
        stroke-width={props.isFold ? FOLD_WIDTH.base + 1 : WIDTH.changed}
        stroke-dasharray={props.dash}
      />
      <text
        class={props.isFold ? 'mr-fold-label' : 'mr-badge'}
        x={span / 2}
        y={8 + (props.isFold ? FOLD_BASELINE : BADGE_BASELINE)}
        fill={props.color}
        stroke={LEGEND_HALO}
      >
        {props.glyph}
      </text>
    </svg>
  )
}

const Legend: Component = () => (
  <div class="legend mr-legend">
    <div class="legend-row">
      <LegendLine color={ADDED_STROKE} glyph="+3 −1" isFold />
      {t('mr.legend.fold')}
    </div>
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
    <div class="legend-row">{t('mr.legend.inner')}</div>
    <div class="legend-row">{t('mr.legend.importer')}</div>
    <div class="legend-row">{t('mr.legend.ghost')}</div>
    <div class="legend-row">{t('mr.legend.select')}</div>
  </div>
)

export const RegionScene: Component<{ model: Model; review: Review }> = (props) => {
  const layout = createMemo(() => layoutRegion(props.model, props.review))
  const focus = createMemo(() => focusOf(props.model, props.review, nav().sel))
  const lines = createMemo(() => routeEdges(layout(), props.review, focus().lines))
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
      <For each={[...props.review.nodes.values()]}>
        {(n) => <Node node={n} layout={layout()} model={props.model} focus={focus()} />}
      </For>
      <g data-testid="mr-folds">
        <For each={layout().folds}>{(mark) => <FoldEdge mark={mark} review={props.review} focus={focus()} />}</For>
      </g>
      <g data-testid="mr-edges">
        <For each={lines()}>{(mark) => <ChangeEdge mark={mark} review={props.review} focus={focus()} />}</For>
      </g>
      <g data-testid="mr-fold-labels">
        <For each={layout().folds}>{(mark) => <FoldLabel mark={mark} focus={focus()} />}</For>
      </g>
    </Canvas>
  )
}
