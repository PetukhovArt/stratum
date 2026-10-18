import { Component, createMemo, For, Show } from 'solid-js'
import { lang, t } from '../../i18n'
import { layerColor } from '../../render/encoding'
import { Model, worst } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { PathText } from '../../shell/path-text'
import { cellOf, Matrix, MatrixCell, matrixOf, pairCells } from './layout'

const LIST_LIMIT = 30
const BLOCK_LIMIT = 8
const BLOCK_NAMES = 4
const PERCENT_DIGITS = 1

const formatNumber = (n: number): string => new Intl.NumberFormat(lang()).format(n)

export const percent = (part: number, whole: number): string => {
  if (!whole) return '—'
  const options = { style: 'percent', minimumFractionDigits: PERCENT_DIGITS, maximumFractionDigits: PERCENT_DIGITS } as const
  return new Intl.NumberFormat(lang(), options).format(part / whole)
}

const layerName = (model: Model, id: number): string => model.layers.find((l) => l.id === id)?.name ?? String(id)
const unitPath = (model: Model, id: number): string => model.containers.get(id)?.path ?? ''

const byWeight = (a: MatrixCell, b: MatrixCell): number => b.breaks - a.breaks || b.count - a.count

const More: Component<{ total: number }> = (props) => (
  <Show when={props.total > LIST_LIMIT}>
    <div class="empty">{t('dsm.more', { n: props.total - LIST_LIMIT })}</div>
  </Show>
)

const ShowInMap: Component<{ level: 'layer' | 'unit'; from: number; to: number }> = (props) => (
  <button
    class="dsm-btn"
    data-testid="dsm-show-in-map"
    onClick={() =>
      go({
        view: 'map',
        mapVariant: props.level === 'unit' ? 'lanes' : 'layers',
        sel: { kind: 'cell', level: props.level, from: props.from, to: props.to },
      })
    }
  >
    {t('map.showInMap')}
  </button>
)

const UnitCell: Component<{ model: Model; cell: MatrixCell }> = (props) => {
  const pairs = createMemo(() =>
    props.model.edges.filter(
      (e) => props.model.unitOf(e.from) === props.cell.from && props.model.unitOf(e.to) === props.cell.to,
    ),
  )
  return (
    <div class="det-section" data-testid="dsm-cell">
      <h4>
        {props.model.unitLabel(props.cell.from)} → {props.model.unitLabel(props.cell.to)}
      </h4>
      <div class="det-path">
        <PathText path={unitPath(props.model, props.cell.from)} />
        <br />→ <PathText path={unitPath(props.model, props.cell.to)} />
      </div>
      <p class="dsm-note">
        {t('dsm.imports', { n: props.cell.count })}
        <Show when={props.cell.breaks}> · <span class="c-err">{t('dsm.breaks', { n: props.cell.breaks })}</span></Show>
      </p>
      <Show when={props.cell.rules.length}>
        <div class="dsm-rules mono">{props.cell.rules.join(', ')}</div>
      </Show>
      <ShowInMap level="unit" from={props.cell.from} to={props.cell.to} />
      <h4>{t('dsm.pairs')}</h4>
      <div class="dsm-rows">
        <For each={pairs().slice(0, LIST_LIMIT)}>
          {(e) => {
            const sev = worst(props.model.findingsOnEdge(e.from, e.to))
            return (
              <button
                class="dsm-row"
                title={`${props.model.modules.get(e.from)?.path}\n→ ${props.model.modules.get(e.to)?.path}`}
                onClick={() => go({ view: 'focus', focus: e.from })}
              >
                <span class="nm mono">
                  {props.model.label(e.from)} → {props.model.label(e.to)}
                </span>
                <Show when={sev}>{(s) => <span class={`dsm-dot sev-${s()}`} data-ch="sev" />}</Show>
              </button>
            )
          }}
        </For>
        <More total={pairs().length} />
      </div>
    </div>
  )
}

const LayerCell: Component<{ model: Model; matrix: Matrix; from: number; to: number }> = (props) => {
  const cell = createMemo(() => cellOf(matrixOf(props.model, 'layer'), props.from, props.to))
  const unitPairs = createMemo(() =>
    props.matrix.level === 'unit' ? pairCells(props.matrix, { from: props.from, to: props.to }).sort(byWeight) : [],
  )
  return (
    <div class="det-section" data-testid="dsm-layer-cell">
      <h4>
        {layerName(props.model, props.from)} → {layerName(props.model, props.to)}
      </h4>
      <Show when={cell()}>
        {(c) => (
          <p class="dsm-note">
            {t('dsm.imports', { n: c().count })}
            <Show when={c().breaks}> · <span class="c-err">{t('dsm.breaks', { n: c().breaks })}</span></Show>
          </p>
        )}
      </Show>
      <ShowInMap level="layer" from={props.from} to={props.to} />
      <Show
        when={props.matrix.level === 'unit'}
        fallback={
          <button
            class="dsm-btn"
            onClick={() => go({ dsmLevel: 'unit', sel: { kind: 'cell', level: 'layer', from: props.from, to: props.to } })}
          >
            {t('dsm.openUnits')}
          </button>
        }
      >
        <h4>{t('dsm.layerPair')}</h4>
        <div class="dsm-rows">
          <For each={unitPairs().slice(0, LIST_LIMIT)}>
            {(c) => (
              <button class="dsm-row" onClick={() => set({ sel: { kind: 'cell', level: 'unit', from: c.from, to: c.to } })}>
                <span class="nm mono">
                  {props.model.unitLabel(c.from)} → {props.model.unitLabel(c.to)}
                </span>
                <span class="n">{c.count}</span>
                <Show when={c.breaks}>
                  <span class="n c-err">{c.breaks}</span>
                </Show>
              </button>
            )}
          </For>
          <More total={unitPairs().length} />
        </div>
      </Show>
    </div>
  )
}

export const DsmDetails: Component<{ model: Model; matrix: Matrix }> = (props) => {
  const sel = () => {
    const s = nav().sel
    return s?.kind === 'cell' ? s : null
  }
  const unitCell = createMemo(() => {
    const s = sel()
    return s?.level === 'unit' && props.matrix.level === 'unit' ? cellOf(props.matrix, s.from, s.to) : null
  })
  const layerPair = createMemo(() => {
    const s = sel()
    return s?.level === 'layer' ? s : null
  })
  const isUnit = () => props.matrix.level === 'unit'
  const d = () => props.matrix.drift
  const blocks = createMemo(() =>
    props.matrix.blocks.slice(0, BLOCK_LIMIT).map((b) => ({
      ...b,
      names: props.matrix.labels.slice(b.start, b.end + 1),
    })),
  )

  return (
    <div class="panel panel-details">
      <div class="panel-hd">
        <span class="panel-title">{t('dsm.drift')}</span>
        <span class="panel-count">{isUnit() ? t('dsm.drift.unit') : t('dsm.drift.layer')}</span>
      </div>
      <div class="panel-body">
        <Show when={unitCell()}>{(c) => <UnitCell model={props.model} cell={c()} />}</Show>
        <Show when={layerPair()}>
          {(p) => <LayerCell model={props.model} matrix={props.matrix} from={p().from} to={p().to} />}
        </Show>
        <div class="det-stats det-stats-grid" data-testid="dsm-stats">
          <div class="stat">
            <div class="stat-val">{formatNumber(d().imports)}</div>
            <div class="stat-lbl">
              {isUnit() ? t('dsm.stat.unitImports', { n: d().imports }) : t('dsm.stat.layerImports', { n: d().imports })}
            </div>
          </div>
          <div class="stat stat-err">
            <div class="stat-val" data-testid="dsm-breaks" data-ch="sev">
              {formatNumber(d().breaks)}
            </div>
            <div class="stat-lbl">{t('dsm.stat.breaks', { n: d().breaks })}</div>
          </div>
          <div class="stat">
            <div class="stat-val" data-testid="dsm-above">
              {percent(d().above, d().imports)}
            </div>
            <div class="stat-lbl">{t('dsm.stat.above')}</div>
          </div>
        </div>
        <p class="dsm-note">{t('dsm.note')}</p>
        <Show when={isUnit() && props.matrix.blocks.length}>
          <div class="det-section">
            <h4>{t('dsm.blocks', { n: props.matrix.blocks.length })}</h4>
            <div class="dsm-rows">
              <For each={blocks()}>
                {(b) => (
                  <div class="dsm-row">
                    <span class="nm mono">
                      {b.names.slice(0, BLOCK_NAMES).join(', ')}
                      {b.names.length > BLOCK_NAMES ? '…' : ''}
                    </span>
                    <span class="n">{t('dsm.blockUnits', { n: b.size })}</span>
                  </div>
                )}
              </For>
            </div>
          </div>
        </Show>
      </div>
    </div>
  )
}

export const DsmOutline: Component<{ model: Model; matrix: Matrix }> = (props) => {
  const layerIndex = createMemo(() => new Map(props.model.layers.map((l, i) => [l.id, i])))
  const swatch = (layer: number) => layerColor(props.model.layerHue(layerIndex().get(layer) ?? 0))
  return (
    <div class="panel">
      <div class="panel-hd">
        <span class="panel-title">{t('dsm.outline')}</span>
        <span class="panel-count">{props.matrix.ids.length}</span>
      </div>
      <div class="outline-tree">
        <For each={props.matrix.labels}>
          {(name, i) => (
            <div class="dsm-ol-row" title={props.matrix.level === 'unit' ? unitPath(props.model, props.matrix.ids[i()]) : name}>
              <span class="dsm-sw" style={{ background: swatch(props.matrix.layers[i()]) }} />
              <span class="nm mono">{name}</span>
            </div>
          )}
        </For>
        <p class="dsm-note dsm-ol-note">{t('dsm.outline.note')}</p>
      </div>
    </div>
  )
}
