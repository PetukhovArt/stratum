import { Accessor, Component, createMemo, createSignal, For, Show } from 'solid-js'
import { BudgetNote } from '../../canvas/budget-note'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { CANVAS_DEFS, EDGE_DASH, EDGE_NEUTRAL, LAYER_TONE, SEV_COLOR } from '../../render/encoding'
import { edgeKey, Model, worst } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { Territory } from '../../types'
import {
  BudgetFold,
  Bundle,
  Column,
  COLUMN,
  ColumnGroup,
  ColumnRow,
  Expanded,
  layoutColumns,
  SHOW_HIDDEN,
} from './columns'
import { FocusCard } from './focus-card'
import { ellipsizePath, monoChars, monoWidth } from '../../render/labels'
import { entryOf, layerName, layerTone, rulesOf, shortRule, curve } from './paint'

const DIM_OPACITY = 0.12
const HIT_WIDTH = 9
const MAX_EXTRA_WIDTH = 3
const DOTTED = '1.5 2.5'
const DASHED = '4 3'
const ROW_TEXT = { nameX: 20, endPad: 20, gap: 8, name: 10, rule: 9 } as const
const COUNT_LABEL = { size: 9, inset: 10, above: 4, below: 11 } as const

interface SceneCtx {
  model: Model
  territory: Territory
  hover: Accessor<number | null>
  setHover: (id: number | null) => void
  toggle: (key: string, isOpen: boolean) => void
}


const selectedPairs = (): ReadonlySet<string> => {
  const sel = nav().sel
  if (sel?.kind === 'imports') return new Set(sel.pairs.map(([a, b]) => edgeKey(a, b)))
  if (sel?.kind === 'edge') return new Set([edgeKey(sel.from, sel.to)])
  return new Set()
}

const countLabelAwayFromFocus = (b: Bundle): { x: number; y: number; anchor: 'start' | 'end' } => {
  const isFarRight = Math.abs(b.x2) >= Math.abs(b.x1)
  const [farY, nearY] = isFarRight ? [b.y2, b.y1] : [b.y1, b.y2]
  const isCurveFromAbove = nearY < farY
  const y = isCurveFromAbove ? farY + COUNT_LABEL.below : farY - COUNT_LABEL.above
  return isFarRight
    ? { x: b.x2 - COUNT_LABEL.inset, y, anchor: 'end' }
    : { x: b.x1 + COUNT_LABEL.inset, y, anchor: 'start' }
}

const BundlePath: Component<{ bundle: Bundle; hover: Accessor<number | null>; selected: Accessor<ReadonlySet<string>> }> = (
  props,
) => {
  const b = props.bundle
  const sev = worst(b.findings)
  const color = sev ? SEV_COLOR[sev] : EDGE_NEUTRAL
  const width = (sev ? 1.6 : 1.1) + Math.min(MAX_EXTRA_WIDTH, Math.log2(b.pairs.length))
  const d = curve(b.x1, b.y1, b.x2, b.y2)
  const isLit = () => {
    const h = props.hover()
    return h === null || b.touches.has(h)
  }
  const isSelected = () => b.pairs.some(([from, to]) => props.selected().has(edgeKey(from, to)))
  const channel = sev ? 'sev' : 'edge'
  const count = countLabelAwayFromFocus(b)
  return (
    <g class="fo-bundle" opacity={isLit() ? 1 : DIM_OPACITY} onClick={() => set({ sel: { kind: 'imports', pairs: b.pairs } })}>
      <path d={d} stroke="transparent" stroke-width={HIT_WIDTH} />
      <path
        d={d}
        data-edge={b.key}
        data-ch={channel}
        data-rule={sev ? rulesOf(b.findings) : undefined}
        stroke={color}
        stroke-opacity={sev || isSelected() ? 0.9 : 0.4}
        stroke-width={isSelected() ? width + 1.5 : width}
        stroke-dasharray={EDGE_DASH[b.kind]}
        marker-end={CANVAS_DEFS.arrow}
      />
      <Show when={b.pairs.length > 1}>
        <text
          data-ch={channel}
          x={count.x}
          y={count.y}
          font-size={String(COUNT_LABEL.size)}
          text-anchor={count.anchor}
          fill={color}
        >
          {b.pairs.length}
        </text>
      </Show>
      <title>{t('focus.edge.imports', { n: b.pairs.length }) + (sev ? ` · ${rulesOf(b.findings)}` : '')}</title>
    </g>
  )
}

const Row: Component<{ ctx: SceneCtx; group: ColumnGroup; row: ColumnRow }> = (props) => {
  const m = props.ctx.model
  const id = props.row.id
  const path = m.modules.get(id)?.path ?? ''
  const name = path.slice(props.group.group.path.length + 1) || m.label(id)
  const edgeSev = worst(props.row.edgeFindings)
  const moduleSev = worst(m.findingsOf(id))
  const isEntry = entryOf(m, id).length > 0
  const firstRule = props.row.edgeFindings[0]?.rule
  const ruleWidth = edgeSev && firstRule ? monoWidth(shortRule(firstRule), ROW_TEXT.rule) + ROW_TEXT.gap : 0
  const nameChars = monoChars(COLUMN.width - ROW_TEXT.nameX - ROW_TEXT.endPad - ruleWidth, ROW_TEXT.name)
  const x0 = props.group.x
  return (
    <g
      class="fo-row"
      data-testid="focus-row"
      data-id={id}
      onMouseEnter={() => props.ctx.setHover(id)}
      onMouseLeave={() => props.ctx.setHover(null)}
      onClick={() => go({ view: 'focus', focus: id })}
    >
      <rect
        x={x0 + 3}
        y={props.row.y - COLUMN.row / 2}
        width={COLUMN.width - 6}
        height={COLUMN.row}
        rx="3"
        fill={props.ctx.hover() === id ? 'oklch(32% 0.02 260 / .8)' : 'transparent'}
      />
      <Show when={isEntry}>
        <text x={x0 + 10} y={props.row.y + 3.5} font-size="9" fill="oklch(80% 0.01 260)">
          ◆
        </text>
      </Show>
      <text
        x={x0 + ROW_TEXT.nameX}
        y={props.row.y + 3.5}
        font-size={String(ROW_TEXT.name)}
        fill={edgeSev ? 'oklch(70% 0.01 260)' : 'oklch(82% 0.006 260)'}
        text-decoration={edgeSev ? 'line-through' : undefined}
      >
        {ellipsizePath(name, nameChars)}
      </text>
      <Show when={edgeSev && firstRule}>
        <text
          data-ch="sev"
          x={x0 + COLUMN.width - ROW_TEXT.endPad}
          y={props.row.y + 3.5}
          font-size={String(ROW_TEXT.rule)}
          text-anchor="end"
          fill={SEV_COLOR[edgeSev!]}
        >
          {shortRule(firstRule!)}
        </text>
      </Show>
      <Show when={moduleSev}>
        <circle data-ch="sev" cx={x0 + COLUMN.width - 11} cy={props.row.y} r="3" fill={SEV_COLOR[moduleSev!]} />
      </Show>
      <title>{path}</title>
    </g>
  )
}

const groupMeta = (m: Model, g: ColumnGroup): string => {
  if (g.group.kind === 'private') return `${t('focus.group.private')} · ${g.ids.length}`
  const container = m.containers.get(g.group.container)
  if (!container || container.kind === 'layer') return String(g.ids.length)
  return `${container.kind.replace('_', '-')} ${t('focus.group.stage', { n: container.stage })} · ${g.ids.length}`
}

const GroupBox: Component<{ ctx: SceneCtx; group: ColumnGroup }> = (props) => {
  const m = props.ctx.model
  const g = props.group
  const hasFindings = g.edgeFindings.length > 0
  const isBoxed = g.group.kind === 'private' || g.group.kind === 'sub_unit' || g.group.kind === 'nested'
  const dash = hasFindings ? DASHED : isBoxed ? DOTTED : undefined
  const closedSev = g.isOpen ? null : worst(g.ids.flatMap((id) => m.findingsOf(id)))
  const meta = groupMeta(m, g)
  const toggle = () => props.ctx.toggle(g.key, !g.isOpen)
  return (
    <g data-testid="focus-group" data-key={g.key}>
      <rect
        class="fo-group-box"
        onClick={toggle}
        x={g.x}
        y={g.y}
        width={COLUMN.width}
        height={g.h}
        rx="6"
        fill={layerTone(m, g.group.layer, LAYER_TONE.tint)}
        stroke={layerTone(m, g.group.layer, hasFindings ? LAYER_TONE.tintEdgeStrong : LAYER_TONE.tintEdge)}
        stroke-dasharray={dash}
      />
      <Show when={hasFindings}>
        <rect x={g.x} y={g.y} width={COLUMN.width} height={g.h} rx="6" fill={CANVAS_DEFS.hatch} pointer-events="none" />
      </Show>
      <g class="fo-group-hd" data-testid="focus-group-head" onClick={toggle}>
        <text x={g.x + 9} y={g.y + 15.5} font-size="9" fill="oklch(55% 0.012 260)">
          {g.isOpen ? '▾' : '▸'}
        </text>
        <text x={g.x + 20} y={g.y + 16} font-size="10.5" font-weight="600" fill="oklch(88% 0.006 260)">
          {ellipsizePath(g.group.name, 25)}
        </text>
        <text x={g.x + COLUMN.width - 9} y={g.y + 16} font-size="9.5" text-anchor="end" fill="oklch(58% 0.012 260)">
          {meta}
        </text>
        <Show when={closedSev}>
          <circle
            data-ch="sev"
            cx={g.x + COLUMN.width - 16 - meta.length * 6}
            cy={g.y + 12}
            r="3"
            fill={SEV_COLOR[closedSev!]}
          />
        </Show>
        <title>{g.group.path}</title>
      </g>
      <For each={g.rows}>{(row) => <Row ctx={props.ctx} group={g} row={row} />}</For>
    </g>
  )
}

const columnTitle = (c: Column): string => {
  const isSecondHop = Math.abs(c.side) === 2
  if (c.side < 0) return t(isSecondHop ? 'focus.col.importedBy2' : 'focus.col.importedBy', { n: c.count })
  return t(isSecondHop ? 'focus.col.imports2' : 'focus.col.imports', { n: c.count })
}

const ColumnView: Component<{ ctx: SceneCtx; column: Column }> = (props) => {
  const c = props.column
  const m = props.ctx.model
  const allowed = c.side > 0 ? props.ctx.territory.may_import_layers : props.ctx.territory.imported_by_layers
  return (
    <g data-testid="focus-column" data-side={c.side}>
      <text
        x={c.side > 0 ? c.minX : c.maxX}
        y={c.top - 14}
        text-anchor={c.side > 0 ? 'start' : 'end'}
        font-size="10"
        fill="oklch(62% 0.012 260)"
        letter-spacing="1"
      >
        {columnTitle(c)}
      </text>
      <For each={c.heads}>
        {(h) => (
          <g>
            <rect x={h.x} y={h.y + 3} width="8" height="8" rx="2" fill={layerTone(m, h.layer)} />
            <text
              x={h.x + 13}
              y={h.y + 11}
              font-size="10"
              font-weight="600"
              letter-spacing=".6"
              fill={layerTone(m, h.layer, LAYER_TONE.label)}
            >
              {layerName(m, h.layer).toUpperCase()}
              {h.isContinued ? ` ${t('focus.col.continued')}` : ''}
            </text>
            <Show when={Math.abs(c.side) === 1 && !allowed.includes(h.layer)}>
              <text x={h.x + COLUMN.width} y={h.y + 11} font-size="9.5" text-anchor="end" fill="oklch(62% 0.012 260)">
                {t('focus.col.outsideLayer')}
              </text>
            </Show>
          </g>
        )}
      </For>
      <For each={c.groups}>{(g) => <GroupBox ctx={props.ctx} group={g} />}</For>
    </g>
  )
}

const foldText = (fold: BudgetFold): string => {
  if (fold.collapsed && fold.hidden) return t('focus.budget.both', { collapsed: fold.collapsed, hidden: fold.hidden })
  if (fold.hidden) return t('focus.budget.hidden', { n: fold.hidden })
  return t('focus.budget.collapsed', { n: fold.collapsed })
}

const FoldNote: Component<{ fold: BudgetFold; showHidden: () => void }> = (props) => (
  <Show when={props.fold.collapsed + props.fold.hidden > 0}>
    <BudgetNote
      text={foldText(props.fold)}
      action={props.fold.hidden ? { label: t('focus.budget.showAll'), run: props.showHidden } : undefined}
    />
  </Show>
)

export const ColumnsScene: Component<{ model: Model; focus: number }> = (props) => {
  const [expansion, setExpansion] = createSignal<{ focus: number; map: Expanded }>({ focus: -1, map: new Map() })
  const [hover, setHover] = createSignal<number | null>(null)
  const expanded = () => {
    const e = expansion()
    return e.focus === props.focus ? e.map : new Map<string, boolean>()
  }
  const toggle = (key: string, isOpen: boolean) =>
    setExpansion({ focus: props.focus, map: new Map(expanded()).set(key, isOpen) })
  const layout = createMemo(() => layoutColumns(props.model, props.focus, nav().hops, expanded()))
  const selected = createMemo(selectedPairs)
  const ctx = (): SceneCtx => ({
    model: props.model,
    territory: props.model.territoryOf(props.focus),
    hover,
    setHover,
    toggle,
  })
  return (
    <Canvas
      bounds={layout().bounds}
      fitKey={`${props.focus}:${nav().hops}`}
      overlay={<FoldNote fold={layout().fold} showHidden={() => toggle(SHOW_HIDDEN, true)} />}
    >
      <g data-testid="focus-columns" fill="none">
        <For each={layout().bundles}>{(b) => <BundlePath bundle={b} hover={hover} selected={selected} />}</For>
      </g>
      <For each={layout().columns}>{(c) => <ColumnView ctx={ctx()} column={c} />}</For>
      <FocusCard model={props.model} id={props.focus} />
    </Canvas>
  )
}
