import { Component, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { sharedNote } from '../../features/shared-mark'
import { Edge, EdgeState } from '../../render/edge'
import { EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR, SHARED_COLOR } from '../../render/encoding'
import { Model } from '../../render/model'
import { ellipsizePath } from '../../render/labels'
import { go, nav, Selection, set } from '../../shell/nav'
import {
  CHIP,
  DEFAULT_SIZE,
  EdgeMark,
  FoldMark,
  FolderBox,
  FolderLayout,
  LaneBox,
  layoutFolder,
  layoutOverview,
  routeFolds,
  metaWidth,
  OPEN,
  routeFolderEdges,
  Size,
} from './layout'
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
const OPACITY = { removed: 0.6, changed: 0.95, same: 0.6, fold: 0.5 } as const
const WIDTH = { changed: 2, same: 1.4 } as const
const NODE_DIMMED = 0.3
const SELECTED_STROKE = 'oklch(97% 0.005 260)'
const FOLD_WIDTH = { base: 1.6, perDoubling: 0.7, max: 5 } as const
const FOLD_STROKE = 'oklch(80% 0.01 260)'
const SIZE_STEP = 50
const TEXT = { name: 'oklch(90% 0.006 260)', quiet: 'oklch(66% 0.01 260)', meta: 'oklch(90% 0.008 260)' } as const

const foldWidth = (f: Fold): number =>
  Math.min(FOLD_WIDTH.max, FOLD_WIDTH.base + Math.log2(f.added + f.removed + 1) * FOLD_WIDTH.perDoubling)

const ruleName = (slug: string): string => slug.slice(slug.lastIndexOf('/') + 1)

export const openFolderOf = (review: Review, sel: Selection | null): string | null => {
  if (sel?.kind === 'box') return review.folderOf.has(sel.key) ? sel.key : null
  if (sel?.kind === 'module') return review.nodes.get(sel.id)?.group.key ?? null
  if (sel?.kind === 'edge') return review.nodes.get(sel.from)?.group.key ?? null
  return null
}

interface RegionFocus {
  lines: ReviewEdge[]
  litLines: ReadonlySet<string> | null
  litNodes: ReadonlySet<number> | null
  edge: string | null
  module: number | null
}

const lit = (edges: readonly ReviewEdge[], also: readonly number[] = []) => ({
  litLines: new Set(edges.map((e) => e.key)),
  litNodes: new Set([...also, ...edges.flatMap((e) => [e.from, e.to])]),
})

const merged = (...lists: readonly (readonly ReviewEdge[])[]): ReviewEdge[] => {
  const byKey = new Map<string, ReviewEdge>()
  for (const list of lists) for (const e of list) byKey.set(e.key, e)
  return [...byKey.values()]
}

const folderFocus = (m: Model, review: Review, key: string, sel: Selection | null): RegionFocus => {
  const inside = (id: number) => review.nodes.get(id)?.group.key === key
  const touches = (e: ReviewEdge) => inside(e.from) || inside(e.to)
  const base = merged(review.changes.filter(touches), review.alwaysDrawn.filter(touches))
  const idle: RegionFocus = { lines: base, litLines: null, litNodes: null, edge: null, module: null }
  if (sel?.kind === 'module' && review.nodes.has(sel.id)) {
    const own = linesOfModule(m, review, sel.id)
    return { ...idle, ...lit(own, [sel.id]), lines: merged(base, own), module: sel.id }
  }
  if (sel?.kind === 'edge') {
    const edge = edgeOf(m, review, sel.from, sel.to)
    if (!edge) return idle
    return { ...idle, ...lit([edge]), lines: merged(base, [edge]), edge: edge.key }
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

const ChangeEdge: Component<{ mark: EdgeMark; review: Review; state: EdgeState; isRuled?: boolean }> = (props) => {
  const e = props.mark.edge
  const sev = e.tag === 'removed' ? null : worstBreak(e.breaks)
  const isChanged = e.tag !== 'same'
  const color = sev ? SEV_COLOR[sev] : e.tag === 'added' ? ADDED_STROKE : EDGE_NEUTRAL
  const badge = () => {
    if (props.isRuled && e.breaks.length) {
      const rule = ruleName(e.breaks[0][0])
      return { text: sev ? rule : `${t('mr.item.fixed')} ${rule}`, class: 'mr-rule-label' }
    }
    return isChanged ? { text: e.tag === 'added' ? '+' : '−', class: 'mr-badge' } : null
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
      state={props.state}
      testId="mr-edge"
      class="mr-edge"
      pathData={{ 'data-edge': e.key, 'data-tag': e.tag, 'data-rule': sev ? rulesOf(e.breaks) : undefined }}
      label={(() => {
        const b = badge()
        return b ? { x: props.mark.mid.x, y: props.mark.mid.y + BADGE_BASELINE, text: b.text, class: b.class, color } : null
      })()}
      hit={false}
      onClick={() => set({ sel: props.state === 'selected' ? null : { kind: 'edge', from: e.from, to: e.to } })}
      title={edgeTitle(props.review, e)}
    />
  )
}

const foldText = (f: Fold): string => `+${f.added} −${f.removed}`

const foldColor = (f: Fold): string => (f.added ? FOLD_STROKE : EDGE_NEUTRAL)

const toggleFold = (selected: boolean, f: Fold): void =>
  set({ sel: selected ? null : { kind: 'link', from: f.from, to: f.to } })

const FoldEdge: Component<{ mark: FoldMark; review: Review; state: EdgeState }> = (props) => {
  const f = props.mark.fold
  const pathOf = (key: string) => props.review.folderOf.get(key)?.path ?? key
  return (
    <Edge
      d={props.mark.d}
      kind="neutral"
      dash={f.added ? undefined : REMOVED_DASH}
      color={foldColor(f)}
      width={foldWidth(f)}
      opacity={OPACITY.fold}
      state={props.state}
      testId="mr-fold"
      class="mr-fold"
      data={{ 'data-fold': f.key }}
      pathData={{ 'data-edge': `fold:${f.key}`, 'data-tag': 'fold' }}
      onClick={() => toggleFold(props.state === 'selected', f)}
      title={`${pathOf(f.from)}\n→ ${pathOf(f.to)}\n${foldText(f)}`}
    />
  )
}

const FoldLabel: Component<{ mark: FoldMark; state: EdgeState }> = (props) => {
  const f = props.mark.fold
  return (
    <text
      class="mr-fold-label"
      data-testid="mr-fold-label"
      data-fold={f.key}
      x={props.mark.mid.x}
      y={props.mark.mid.y + FOLD_BASELINE}
      fill={foldColor(f)}
      stroke={LABEL_HALO}
      opacity={props.state === 'dimmed' ? LABEL_DIMMED : 1}
      onClick={() => toggleFold(props.state === 'selected', f)}
    >
      {foldText(f)}
    </text>
  )
}

const Lane: Component<{ lane: LaneBox; model: Model }> = (props) => {
  const l = props.lane
  return (
    <g>
      <rect
        x={l.x}
        y={l.y}
        width={l.w}
        height={l.h}
        rx="8"
        fill={layerTint(props.model, l.layer, LAYER_TONE.band)}
        stroke={layerTint(props.model, l.layer, LAYER_TONE.tintEdge)}
      />
      <rect x={l.x} y={l.y} width="4" height={l.h} rx="2" fill={layerTint(props.model, l.layer)} />
      <text
        x={l.x + 12}
        y={l.y + 8 + CHIP.h / 2 + 4}
        font-size="11"
        font-weight="600"
        letter-spacing="1"
        fill={layerTint(props.model, l.layer, LAYER_TONE.label)}
      >
        {l.name}
      </text>
    </g>
  )
}

const ChipMeta: Component<{ box: FolderBox; review: Review; y: number }> = (props) => {
  const text = props.box.text
  const f = props.box.folder
  const isStage = f.container !== null && props.review.stageOf.has(f.container)
  const parts = () => {
    let x = props.box.x + props.box.w - CHIP.padX - metaWidth(text) + CHIP.metaGap
    const out: { x: number; kind: 'counts' | 'breaks' | 'stage' | 'gone'; text: string }[] = []
    const push = (kind: (typeof out)[number]['kind'], s: string, extra = 0) => {
      if (!s) return
      out.push({ x, kind, text: s })
      x += extra + s.length * CHIP.metaChar + CHIP.metaGap
    }
    push('counts', text.counts)
    push('breaks', text.breaks, CHIP.dot)
    push('stage', isStage ? text.stage : '')
    push('gone', text.gone)
    return out
  }
  const fill = (kind: string) =>
    kind === 'breaks' ? SEV_COLOR[f.worst ?? 'error'] : kind === 'stage' ? STAGE_STROKE : kind === 'gone' ? TEXT.quiet : TEXT.meta
  return (
    <For each={parts()}>
      {(p) => (
        <>
          <Show when={p.kind === 'breaks'}>
            <circle cx={p.x + 3.5} cy={props.y - 4} r="3.5" fill={fill(p.kind)} data-ch="sev" />
          </Show>
          <text
            x={p.kind === 'breaks' ? p.x + CHIP.dot : p.x}
            y={props.y}
            font-size="11"
            font-family="var(--mono)"
            font-weight={p.kind === 'counts' || p.kind === 'breaks' ? '700' : '400'}
            fill={fill(p.kind)}
            data-testid={`mr-folder-${p.kind}`}
            data-ch={p.kind === 'breaks' ? 'sev' : undefined}
          >
            {p.text}
          </text>
        </>
      )}
    </For>
  )
}

const FolderChip: Component<{
  box: FolderBox
  model: Model
  review: Review
  isDimmed: boolean
  onClick: () => void
  onHover?: (over: boolean) => void
}> = (props) => {
  const b = props.box
  const f = b.folder
  const isStage = f.container !== null && props.review.stageOf.has(f.container)
  const hasChanges = f.added + f.removed > 0
  return (
    <g
      class="mr-folder"
      data-testid="mr-folder"
      data-folder={f.key}
      data-state={props.isDimmed ? 'dimmed' : undefined}
      opacity={props.isDimmed ? NODE_DIMMED : undefined}
      onClick={() => props.onClick()}
      onMouseEnter={() => props.onHover?.(true)}
      onMouseLeave={() => props.onHover?.(false)}
    >
      <rect
        x={b.x}
        y={b.y}
        width={b.w}
        height={b.h}
        rx="5"
        fill={layerTint(props.model, f.layer, LAYER_TONE.tint)}
        stroke={isStage ? STAGE_STROKE : layerTint(props.model, f.layer, LAYER_TONE.tintEdgeStrong)}
        stroke-width={isStage ? 1.8 : 1}
        stroke-dasharray={f.isPrivate ? '4 3' : undefined}
      />
      <text x={b.x + CHIP.padX} y={b.y + CHIP.h / 2 + 4} font-size="12" fill={hasChanges ? TEXT.name : TEXT.quiet}>
        {b.text.name}
      </text>
      <ChipMeta box={b} review={props.review} y={b.y + CHIP.h / 2 + 4} />
      <title>{f.path}</title>
    </g>
  )
}

const OpenFolder: Component<{ box: FolderBox; model: Model; review: Review }> = (props) => {
  const b = props.box
  const f = b.folder
  const isStage = f.container !== null && props.review.stageOf.has(f.container)
  return (
    <g data-testid="mr-open-folder" data-folder={f.key}>
      <rect
        x={b.x}
        y={b.y}
        width={b.w}
        height={b.h}
        rx="7"
        fill={layerTint(props.model, f.layer, LAYER_TONE.tint)}
        stroke={isStage ? STAGE_STROKE : layerTint(props.model, f.layer, LAYER_TONE.tintEdgeStrong)}
        stroke-width={isStage ? 1.8 : 1.2}
        stroke-dasharray={f.isPrivate ? '4 3' : undefined}
      />
      <text x={b.x + CHIP.padX} y={b.y + OPEN.head / 2 + 3} font-size="12.5" font-weight="600" fill={TEXT.name}>
        {b.text.name}
      </text>
      <ChipMeta box={b} review={props.review} y={b.y + OPEN.head / 2 + 3} />
      <title>{f.path}</title>
    </g>
  )
}

const Node: Component<{ node: ReviewNode; layout: FolderLayout; model: Model; focus: RegionFocus }> = (props) => {
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
      <text x="8" y="15" font-size="11" fill={n.isGhost ? 'oklch(68% 0.01 260)' : 'oklch(90% 0.005 260)'}>
        {ellipsizePath(labelOf(props.model, n.id, n.path), LABEL_CHARS)}
      </text>
      <Show when={n.isImporter}>
        <text x={r.w - 8} y="15" font-size="10" text-anchor="end" font-weight="700" fill="oklch(88% 0.01 260)">
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

const OverviewLegend: Component = () => (
  <div class="legend mr-legend">
    <div class="legend-row">
      <LegendLine color={ADDED_STROKE} glyph="+3 −1" isFold />
      {t('mr.legend.fold')}
    </div>
    <div class="legend-row">
      <span class="mr-lg-sev" data-ch="sev" />
      {t('mr.legend.break')}
    </div>
    <div class="legend-row">{t('mr.legend.folder')}</div>
    <div class="legend-row">{t('mr.legend.open')}</div>
  </div>
)

const FolderLegend: Component = () => (
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
    <div class="legend-row">{t('mr.legend.select')}</div>
  </div>
)

const Overview: Component<{ model: Model; review: Review; size: Size }> = (props) => {
  const layout = createMemo(() => layoutOverview(props.model, props.review, props.size))
  const fold = createMemo(() => {
    const sel = nav().sel
    return sel?.kind === 'link' ? props.review.folds.find((f) => f.from === sel.from && f.to === sel.to) ?? null : null
  })
  const [hovered, setHovered] = createSignal<string | null>(null)
  const shown = createMemo(() => {
    const f = fold()
    if (f) return [f]
    const key = hovered()
    return key ? props.review.folds.filter((x) => x.from === key || x.to === key) : []
  })
  const folds = createMemo(() => routeFolds(layout(), shown()))
  const foldState = (f: Fold): EdgeState => (fold() !== null && fold()!.key === f.key ? 'selected' : 'idle')
  const isLit = (key: string) => shown().some((f) => f.from === key || f.to === key)
  const isDimmed = (key: string) => shown().length > 0 && !isLit(key)
  return (
    <Canvas bounds={layout().bounds} fitKey={layout()} legend={<OverviewLegend />}>
      <For each={layout().lanes}>{(l) => <Lane lane={l} model={props.model} />}</For>
      <g data-testid="mr-edges">
        <For each={layout().breaks}>
          {(mark) => <ChangeEdge mark={mark} review={props.review} state={shown().length ? 'dimmed' : 'idle'} isRuled />}
        </For>
      </g>
      <g data-testid="mr-folds">
        <For each={folds()}>{(mark) => <FoldEdge mark={mark} review={props.review} state={foldState(mark.fold)} />}</For>
      </g>
      <For each={layout().boxes}>
        {(b) => (
          <FolderChip
            box={b}
            model={props.model}
            review={props.review}
            isDimmed={isDimmed(b.folder.key)}
            onClick={() => set({ sel: { kind: 'box', key: b.folder.key } })}
            onHover={(over) => setHovered(over ? b.folder.key : null)}
          />
        )}
      </For>
      <g data-testid="mr-fold-labels">
        <For each={folds()}>{(mark) => <FoldLabel mark={mark} state="selected" />}</For>
      </g>
    </Canvas>
  )
}

const Crumbs: Component<{ path: string }> = (props) => (
  <div class="mr-crumbs">
    <button class="mr-btn" data-testid="mr-crumb-all" onClick={() => set({ sel: null })}>
      ← {t('mr.crumb.all')}
    </button>
    <span class="mono mr-crumb-path">{props.path}</span>
  </div>
)

const FolderScene: Component<{ model: Model; review: Review; folder: string; size: Size }> = (props) => {
  const layout = createMemo(() => layoutFolder(props.model, props.review, props.folder, props.size))
  const focus = createMemo(() => folderFocus(props.model, props.review, props.folder, nav().sel))
  const lines = createMemo(() => routeFolderEdges(layout(), focus().lines))
  const litFolders = createMemo(() => {
    const nodes = focus().litNodes
    return nodes ? new Set([...nodes].map((id) => props.review.nodes.get(id)?.group.key)) : null
  })
  return (
    <Canvas
      bounds={layout().bounds}
      fitKey={layout()}
      legend={<FolderLegend />}
      overlay={<Crumbs path={layout().open.folder.path} />}
    >
      <For each={layout().lanes}>{(l) => <Lane lane={l} model={props.model} />}</For>
      <OpenFolder box={layout().open} model={props.model} review={props.review} />
      <For each={layout().boxes}>
        {(b) => (
          <FolderChip
            box={b}
            model={props.model}
            review={props.review}
            isDimmed={litFolders() !== null && !litFolders()!.has(b.folder.key)}
            onClick={() => set({ sel: { kind: 'box', key: b.folder.key } })}
          />
        )}
      </For>
      <For each={layout().open.folder.members}>
        {(id) => <Node node={props.review.nodes.get(id)!} layout={layout()} model={props.model} focus={focus()} />}
      </For>
      <g data-testid="mr-edges">
        <For each={lines()}>
          {(mark) => <ChangeEdge mark={mark} review={props.review} state={lineState(focus(), mark.edge.key)} />}
        </For>
      </g>
    </Canvas>
  )
}

const sizeOf = (el: HTMLElement): Size => {
  const r = el.getBoundingClientRect()
  return r.width && r.height
    ? { w: Math.round(r.width / SIZE_STEP) * SIZE_STEP, h: Math.round(r.height / SIZE_STEP) * SIZE_STEP }
    : DEFAULT_SIZE
}

export const RegionScene: Component<{ model: Model; review: Review }> = (props) => {
  let wrap!: HTMLDivElement
  const [size, setSize] = createSignal<Size>(DEFAULT_SIZE, { equals: (a, b) => a.w === b.w && a.h === b.h })
  onMount(() => {
    const ro = new ResizeObserver(() => setSize(sizeOf(wrap)))
    ro.observe(wrap)
    setSize(sizeOf(wrap))
    onCleanup(() => ro.disconnect())
  })
  const open = createMemo(() => openFolderOf(props.review, nav().sel))
  return (
    <div class="mr-region" ref={wrap}>
      <Show when={open()} keyed fallback={<Overview model={props.model} review={props.review} size={size()} />}>
        {(key) => <FolderScene model={props.model} review={props.review} folder={key} size={size()} />}
      </Show>
    </div>
  )
}
