import { Component, createMemo, createSignal, Show } from 'solid-js'
import { Canvas } from '../../canvas/canvas'
import { t } from '../../i18n'
import { Model } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { ViewDef, ViewProps } from '../view-def'
import './dsm.css'
import { cellOf, MatrixCell, matrixOf } from './layout'
import { MatrixMarks } from './matrix'
import { DsmDetails, DsmOutline, percent } from './panels'

const TOOLTIP_OFFSET = 14

const currentMatrix = (model: Model) => matrixOf(model, nav().dsmLevel)

const Stage: Component<ViewProps> = (props) => {
  const matrix = createMemo(() => currentMatrix(props.model))
  const [hover, setHover] = createSignal<{ cell: MatrixCell; x: number; y: number } | null>(null)
  let wrap!: HTMLDivElement

  const sel = () => {
    const s = nav().sel
    return s?.kind === 'cell' ? s : null
  }
  const selected = createMemo(() => {
    const s = sel()
    return s && s.level === matrix().level ? cellOf(matrix(), s.from, s.to) : null
  })
  const pair = createMemo(() => {
    const s = sel()
    return s && s.level === 'layer' && matrix().level === 'unit' ? { from: s.from, to: s.to } : null
  })

  const onHover = (cell: MatrixCell | null, e: MouseEvent) => {
    if (!cell) {
      setHover(null)
      return
    }
    const r = wrap.getBoundingClientRect()
    setHover({ cell, x: e.clientX - r.left, y: e.clientY - r.top })
  }
  const onPick = (cell: MatrixCell) => {
    if (matrix().level === 'layer') {
      go({ dsmLevel: 'unit', sel: { kind: 'cell', level: 'layer', from: cell.from, to: cell.to } })
      return
    }
    set({ sel: { kind: 'cell', level: 'unit', from: cell.from, to: cell.to } })
  }
  const labelOf = (id: number) => matrix().labels[matrix().ids.indexOf(id)] ?? String(id)

  return (
    <div class="dsm-stage" ref={wrap}>
      <div class="dsm-drift" data-testid="dsm-drift">
        <span class="lbl">{t('dsm.drift')}</span>
        <span>
          {t('dsm.header', {
            breaks: matrix().drift.breaks,
            above: percent(matrix().drift.above, matrix().drift.imports),
          })}
        </span>
      </div>
      <div class="dsm-canvas">
        <Canvas bounds={matrix().bounds} fitKey={matrix()} legend={<Legend hasBlocks={matrix().blocks.length > 0} />}>
          <MatrixMarks
            model={props.model}
            matrix={matrix()}
            selected={selected()}
            hovered={hover()?.cell ?? null}
            pair={pair()}
            onHover={onHover}
            onPick={onPick}
          />
        </Canvas>
        <Show when={hover()}>
          {(h) => (
            <div class="dsm-tip" style={{ left: `${h().x + TOOLTIP_OFFSET}px`, top: `${h().y + TOOLTIP_OFFSET}px` }}>
              <div class="mono">
                {labelOf(h().cell.from)} → {labelOf(h().cell.to)}
              </div>
              <div>{t('dsm.imports', { n: h().cell.count })}</div>
              <Show when={h().cell.breaks}>
                <div class="c-err">{t('dsm.breaks', { n: h().cell.breaks })}</div>
              </Show>
              <Show when={h().cell.rules.length}>
                <div class="mono dim">{h().cell.rules.join(', ')}</div>
              </Show>
              <Show when={h().cell.isAbove}>
                <div class="dim">{t('dsm.aboveCell')}</div>
              </Show>
            </div>
          )}
        </Show>
      </div>
    </div>
  )
}

const Legend: Component<{ hasBlocks: boolean }> = (props) => (
  <div class="legend dsm-legend">
    <div class="legend-row">
      <span class="mono dsm-lg">r→c</span>
      {t('dsm.legend.rowCol')}
    </div>
    <div class="legend-row">
      <span class="dsm-lg dsm-lg-above" />
      {t('dsm.legend.above')}
    </div>
    <div class="legend-row">
      <span class="dsm-lg dsm-lg-mark" data-ch="sev" />
      {t('dsm.legend.mark')}
    </div>
    <Show when={props.hasBlocks}>
      <div class="legend-row">
        <span class="dsm-lg dsm-lg-block" />
        {t('dsm.legend.block')}
      </div>
    </Show>
  </div>
)

const Details: Component<ViewProps> = (props) => (
  <DsmDetails model={props.model} matrix={currentMatrix(props.model)} />
)

const Outline: Component<ViewProps> = (props) => (
  <DsmOutline model={props.model} matrix={currentMatrix(props.model)} />
)

export const dsmView: ViewDef = {
  id: 'dsm',
  key: '2',
  title: 'view.dsm',
  Stage,
  Details,
  Outline,
  variants: {
    key: 'dsmLevel',
    options: [
      { id: 'layer', title: 'dsm.level.layer' },
      { id: 'unit', title: 'dsm.level.unit' },
    ],
  },
}
