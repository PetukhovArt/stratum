import { Component, createEffect, createMemo, For, on, Show } from 'solid-js'
import { useCanvas } from '../../canvas/canvas'
import { layerColor, SEV_COLOR } from '../../render/encoding'
import { Model } from '../../render/model'
import { Bounds, fitViewport } from '../../render/viewport'
import { Matrix, MatrixCell, pairCells } from './layout'

const LABEL_GAP = 12
const STRIP = { offset: 6, width: 4 } as const
const LABEL = {
  layer: { size: 11, chars: 18 },
  unit: { size: 7.5, chars: 20 },
} as const
const COUNT_TEXT_SIZE = 10
const COLUMN_LABEL_ANGLE = -60
const MARK_OPACITY = { base: 0.55, range: 0.45 } as const
const GREY = { lightness: 40, range: 45, chroma: 0.01, aboveChroma: 0.02 } as const

export interface LayerPair {
  from: number
  to: number
}

const truncate = (text: string, max: number): string => (text.length > max ? `${text.slice(0, max - 1)}…` : text)

const rectPath = (x: number, y: number, w: number, h: number): string => `M${x} ${y}h${w}v${h}h${-w}z`

const cellsBounds = (matrix: Matrix, cells: readonly MatrixCell[]): Bounds | null => {
  if (!cells.length) return null
  const s = matrix.cell
  const rows = cells.map((c) => c.row)
  const cols = cells.map((c) => c.col)
  const x = Math.min(...cols) * s
  const y = Math.min(...rows) * s
  return { x, y, w: (Math.max(...cols) + 1) * s - x, h: (Math.max(...rows) + 1) * s - y }
}

export const MatrixMarks: Component<{
  model: Model
  matrix: Matrix
  selected: MatrixCell | null
  hovered: MatrixCell | null
  pair: LayerPair | null
  onHover: (cell: MatrixCell | null, e: MouseEvent) => void
  onPick: (cell: MatrixCell) => void
}> = (props) => {
  const canvas = useCanvas()
  const s = () => props.matrix.cell
  const span = () => props.matrix.ids.length * s()
  const label = () => LABEL[props.matrix.level]
  const maxCount = createMemo(() => Math.max(1, ...props.matrix.cells.map((c) => c.count)))
  const layerIndex = createMemo(() => new Map(props.model.layers.map((l, i) => [l.id, i])))
  const hue = (layer: number) => props.model.layerHue(layerIndex().get(layer) ?? 0)

  const strips = createMemo(() => {
    const byLayer = new Map<number, { rows: string; cols: string }>()
    for (const run of props.matrix.runs) {
      const acc = byLayer.get(run.layer) ?? { rows: '', cols: '' }
      const from = run.start * s()
      const len = (run.end - run.start + 1) * s()
      acc.rows += rectPath(-STRIP.offset, from, STRIP.width, len)
      acc.cols += rectPath(from, -STRIP.offset, len, STRIP.width)
      byLayer.set(run.layer, acc)
    }
    return [...byLayer.entries()].map(([layer, d]) => ({ layer, d: d.rows + d.cols }))
  })

  const pairOutline = createMemo(() => {
    const pair = props.pair
    if (!pair) return null
    const cells = pairCells(props.matrix, pair)
    return {
      d: cells.map((c) => rectPath(c.col * s(), c.row * s(), s(), s())).join(''),
      bounds: cellsBounds(props.matrix, cells),
    }
  })

  createEffect(
    on([() => pairOutline()?.bounds ?? null, canvas.rect], ([bounds, rect], previous) => {
      const wasZoomedToPair = Boolean(previous?.[0])
      if (!rect) return
      if (bounds) {
        canvas.setViewport(fitViewport(bounds, { w: rect.width, h: rect.height }))
        return
      }
      if (wasZoomedToPair) canvas.fit()
    }),
  )

  const fill = (c: MatrixCell) => {
    const t = Math.log(c.count + 1) / Math.log(maxCount() + 1)
    if (c.mark) return { fill: SEV_COLOR[c.mark], opacity: MARK_OPACITY.base + t * MARK_OPACITY.range }
    const chroma = c.isAbove ? GREY.aboveChroma : GREY.chroma
    return { fill: `oklch(${GREY.lightness + t * GREY.range}% ${chroma} 260)`, opacity: 1 }
  }

  const cellAt = (e: MouseEvent): MatrixCell | null => {
    const index = (e.target as Element).getAttribute?.('data-i')
    return index === null || index === undefined ? null : (props.matrix.cells[Number(index)] ?? null)
  }

  return (
    <g class="dsm" data-level={props.matrix.level}>
      <path class="dsm-above" d={`M0 0L${span()} 0L${span()} ${span()}Z`} />
      <line class="dsm-diag" x1={0} y1={0} x2={span()} y2={span()} />
      <For each={strips()}>{(strip) => <path d={strip.d} fill={layerColor(hue(strip.layer))} />}</For>
      <rect class="dsm-frame" x={0} y={0} width={span()} height={span()} />
      <Show when={props.hovered}>
        {(h) => (
          <>
            <rect class="dsm-cross" x={0} y={h().row * s()} width={span()} height={s()} />
            <rect class="dsm-cross" x={h().col * s()} y={0} width={s()} height={span()} />
          </>
        )}
      </Show>
      <g
        class="dsm-cells"
        onMouseMove={(e) => props.onHover(cellAt(e), e)}
        onMouseLeave={(e) => props.onHover(null, e)}
        onClick={(e) => {
          const cell = cellAt(e)
          if (cell) props.onPick(cell)
        }}
      >
        <For each={props.matrix.cells}>
          {(c, i) => {
            const f = fill(c)
            const hasFindings = c.rules.length > 0
            return (
              <rect
                data-i={i()}
                data-ch={c.mark ? 'sev' : undefined}
                data-edge={hasFindings ? `${props.matrix.level}:${c.key}` : undefined}
                data-rule={hasFindings ? c.rules.join(' ') : undefined}
                x={c.col * s() + 0.5}
                y={c.row * s() + 0.5}
                width={s() - 1}
                height={s() - 1}
                rx={props.matrix.level === 'layer' ? 3 : 1}
                fill={f.fill}
                fill-opacity={f.opacity}
              />
            )
          }}
        </For>
      </g>
      <Show when={props.matrix.level === 'layer'}>
        <For each={props.matrix.cells}>
          {(c) => (
            <text
              class={c.mark ? 'dsm-count on-mark' : 'dsm-count'}
              x={c.col * s() + s() / 2}
              y={c.row * s() + s() / 2 + COUNT_TEXT_SIZE / 3}
              font-size={String(COUNT_TEXT_SIZE)}
            >
              {c.count}
            </text>
          )}
        </For>
      </Show>
      <For each={props.matrix.blocks}>
        {(b) => (
          <rect
            class="dsm-block"
            x={b.start * s() - 1}
            y={b.start * s() - 1}
            width={(b.end - b.start + 1) * s() + 2}
            height={(b.end - b.start + 1) * s() + 2}
          />
        )}
      </For>
      <Show when={pairOutline()}>{(outline) => <path class="dsm-pair" d={outline().d} />}</Show>
      <Show when={props.selected}>
        {(c) => <rect class="dsm-sel" x={c().col * s()} y={c().row * s()} width={s()} height={s()} />}
      </Show>
      <g class="dsm-labels" font-size={String(label().size)}>
        <For each={props.matrix.labels}>
          {(name, i) => (
            <>
              <text x={-LABEL_GAP} y={i() * s() + s() / 2 + label().size / 3} text-anchor="end">
                {truncate(name, label().chars)}
              </text>
              <text
                transform={`translate(${i() * s() + s() / 2 + label().size / 3},${-LABEL_GAP}) rotate(${COLUMN_LABEL_ANGLE})`}
              >
                {truncate(name, label().chars)}
              </text>
            </>
          )}
        </For>
      </g>
    </g>
  )
}
