import { Component, createMemo, createSignal, For } from 'solid-js'
import { Canvas, exportSvg } from '../../canvas/canvas'
import { t } from '../../i18n'
import { SEV_COLOR } from '../../render/encoding'
import { Model } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { DriftReport } from './drift'
import { describeItem } from './items'
import { trunc } from './paint'
import { Review } from './review'

const EXPORTED_LIMIT = { w: 800, h: 600 } as const
const EXPORT_PAD = 24
export const CARD = {
  w: 740,
  maxH: EXPORTED_LIMIT.h - EXPORT_PAD * 2,
  top: 88,
  row: 44,
  foot: 34,
} as const

const MAIN_CHARS = 78
const SUB_CHARS = 92
const PILL_W = 92
const PILL_CHARS = 15
const TEXT_X = 108
const EXPORT_NAME = 'architecture-drift'

export const cardRows = (itemCount: number): number =>
  Math.min(itemCount, Math.floor((CARD.maxH - CARD.top - CARD.foot) / CARD.row))

export const cardHeight = (rows: number): number => CARD.top + rows * CARD.row + CARD.foot

const CardSvg: Component<{ model: Model; review: Review; drift: DriftReport }> = (props) => {
  const rows = createMemo(() => cardRows(props.review.items.length))
  const height = createMemo(() => cardHeight(rows()))
  const items = createMemo(() => props.review.items.slice(0, rows()).map((it) => describeItem(props.model, it)))
  const more = () => props.review.items.length - rows()
  const c = () => props.review.counts
  const selectedKey = () => {
    const sel = nav().sel
    return sel?.kind === 'edge' ? `${sel.from}>${sel.to}` : null
  }
  const summary = () =>
    [
      t('mr.card.imports', { added: c().added, removed: c().removed }),
      `${c().cycles} ${t('mr.stat.cycles', { n: c().cycles })}`,
      `${c().stages} ${t('mr.stat.stages', { n: c().stages })}`,
    ].join(' · ')
  return (
    <Canvas bounds={{ x: 0, y: 0, w: CARD.w, h: height() }} fitKey="mr-card">
      <g data-testid="mr-card" data-size={`${CARD.w}x${height()}`}>
        <rect width={CARD.w} height={height()} rx="12" fill="oklch(19% 0.013 260)" stroke="oklch(36% 0.012 260)" />
        <text x="24" y="34" font-size="15" font-weight="600" fill="oklch(95% 0.005 260)">
          {t('mr.card.title')}
        </text>
        <text x={CARD.w - 24} y="34" font-size="11" text-anchor="end" fill="oklch(62% 0.012 260)">
          {props.drift.base.ref} · {props.drift.base.sha}
        </text>
        <text x="24" y="62" font-size="12" fill="oklch(84% 0.006 260)">
          <tspan data-ch={c().breaking ? 'sev' : undefined} fill={c().breaking ? SEV_COLOR.error : undefined}>
            {c().breaking} {t('mr.stat.breaking', { n: c().breaking })}
          </tspan>
          {` · ${summary()}`}
        </text>
        <line x1="24" y1="76" x2={CARD.w - 24} y2="76" stroke="oklch(34% 0.012 260)" />
        <For each={items()}>
          {(it, i) => {
            const color = it.severity ? SEV_COLOR[it.severity] : 'oklch(70% 0.01 260)'
            const channel = it.severity ? 'sev' : 'diff'
            const isOn = () => it.edge !== null && selectedKey() === it.edge.key
            const toggle = () => {
              if (!it.edge) return
              set({ sel: isOn() ? null : { kind: 'edge', from: it.edge.from, to: it.edge.to } })
            }
            return (
              <g transform={`translate(24,${CARD.top + i() * CARD.row})`} class="mr-card-row" onClick={toggle}>
                <rect
                  x="-8"
                  y="-4"
                  width={CARD.w - 32}
                  height={CARD.row - 6}
                  rx="6"
                  fill={isOn() ? 'oklch(26% 0.016 260)' : 'transparent'}
                />
                <rect
                  x="0"
                  y="4"
                  width={PILL_W}
                  height="22"
                  rx="11"
                  fill="none"
                  stroke={color}
                  stroke-dasharray={it.isFixed ? '3 2' : undefined}
                  data-ch={channel}
                />
                <text x={PILL_W / 2} y="19" font-size="10" text-anchor="middle" fill={color} data-ch={channel}>
                  {trunc(it.pill, PILL_CHARS)}
                </text>
                <text x={TEXT_X} y="14" font-size="11" fill="oklch(92% 0.005 260)">
                  {trunc(it.main, MAIN_CHARS)}
                </text>
                <text x={TEXT_X} y="30" font-size="9.5" fill="oklch(62% 0.012 260)">
                  {trunc(it.sub, SUB_CHARS)}
                </text>
              </g>
            )
          }}
        </For>
        <text x="24" y={height() - 14} font-size="9.5" fill="oklch(52% 0.012 260)">
          {more() > 0 ? `${t('mr.card.more', { n: more() })} · ` : ''}
          {CARD.w}×{height()} px
        </text>
      </g>
    </Canvas>
  )
}

export const CardScene: Component<{ model: Model; review: Review; drift: DriftReport }> = (props) => {
  const [isCopied, setCopied] = createSignal(false)
  const copy = async () => {
    await navigator.clipboard.writeText(props.drift.markdown)
    setCopied(true)
  }
  return (
    <div class="mr-card-stage">
      <div class="mr-card-canvas">
        <CardSvg model={props.model} review={props.review} drift={props.drift} />
      </div>
      <aside class="mr-md">
        <div class="mr-md-hd">
          <span>{t('mr.markdown')}</span>
          <span class="mr-md-btns">
            <button class="mr-btn" data-testid="mr-copy" onClick={() => void copy()}>
              {isCopied() ? t('mr.copied') : t('mr.copy')}
            </button>
            <button class="mr-btn" data-testid="mr-export" onClick={() => exportSvg(EXPORT_NAME)}>
              {t('mr.exportSvg')}
            </button>
          </span>
        </div>
        <pre class="mr-md-body" data-testid="mr-markdown">
          {props.drift.markdown}
        </pre>
      </aside>
    </div>
  )
}
