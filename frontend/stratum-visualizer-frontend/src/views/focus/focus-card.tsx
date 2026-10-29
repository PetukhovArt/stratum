import { Component, For, Show } from 'solid-js'
import { sharedAttr, sharedNote } from '../../features/shared-mark'
import { t } from '../../i18n'
import { LAYER_TONE, SEV_COLOR } from '../../render/encoding'
import { Model } from '../../render/model'
import { CARD } from './columns'
import { ellipsizePath, trunc } from '../../render/labels'
import { entryOf, layerName, layerTone, severityCounts } from './paint'

const SEV_STEP = 92
const HALO = 4

export const FocusCard: Component<{ model: Model; id: number }> = (props) => {
  const m = () => props.model
  const module = () => m().modules.get(props.id)!
  const units = () => m().chain(props.id).filter((c) => c.kind !== 'layer')
  const crumbs = () => [layerName(m(), module().layer), ...units().map((c) => c.name)].join(' › ')
  const dir = () => `${module().path.split('/').slice(0, -1).join('/')}/`
  const meta = () => {
    const parts = [t('focus.card.purity', { n: module().purity })]
    const inner = units().at(-1)
    if (inner) parts.push(t('focus.card.stage', { n: inner.stage }))
    const entries = entryOf(m(), props.id)
    if (entries.length) parts.push(t('focus.card.entry', { names: entries.join(', ') }))
    if (typeof module().zone === 'number') parts.push(t('focus.card.private'))
    if (m().isSharedByChosenFeatures(props.id)) parts.push(t('features.shared'))
    return parts.join(' · ')
  }
  const counts = () => severityCounts(m().findingsOf(props.id))
  const x = -CARD.w / 2
  const y = -CARD.h / 2
  return (
    <g class="fo-card" data-testid="focus-card" data-id={props.id} data-shared={sharedAttr(m(), props.id)}>
      <rect
        x={x - HALO}
        y={y - HALO}
        width={CARD.w + HALO * 2}
        height={CARD.h + HALO * 2}
        rx="12"
        fill="none"
        stroke={layerTone(m(), module().layer, LAYER_TONE.cardHalo)}
      />
      <rect
        x={x}
        y={y}
        width={CARD.w}
        height={CARD.h}
        rx="9"
        fill="oklch(23% 0.016 260)"
        stroke={layerTone(m(), module().layer, LAYER_TONE.card)}
        stroke-width="1.4"
      />
      <rect x={x} y={y} width="4" height={CARD.h} rx="2" fill={layerTone(m(), module().layer)} />
      <text x={x + 16} y={y + 22} font-size="10" fill={layerTone(m(), module().layer, LAYER_TONE.label)}>
        {trunc(crumbs(), 40)}
      </text>
      <text x={x + 16} y={y + 46} font-size="15" font-weight="600" fill="oklch(95% 0.005 260)">
        {ellipsizePath(m().label(props.id), 28)}
      </text>
      <text x={x + 16} y={y + 64} font-size="9.5" fill="oklch(60% 0.012 260)">
        {ellipsizePath(dir(), 44)}
      </text>
      <text x={x + 16} y={y + 88} font-size="10" fill="oklch(78% 0.008 260)">
        {trunc(meta(), 44)}
      </text>
      <g transform={`translate(${x + 16},${y + 108})`}>
        <For each={counts()}>
          {([sev, n], i) => (
            <g data-ch="sev" transform={`translate(${i() * SEV_STEP},0)`}>
              <circle cx="4" cy="-3" r="3.5" fill={SEV_COLOR[sev]} />
              <text x="12" y="0" font-size="10" fill={SEV_COLOR[sev]}>
                {t(`focus.sev.${sev}`, { n })}
              </text>
            </g>
          )}
        </For>
        <Show when={counts().length === 0}>
          <text x="0" y="0" font-size="10" fill="oklch(55% 0.012 260)">
            {t('focus.card.clean')}
          </text>
        </Show>
      </g>
      <title>{module().path + sharedNote(m(), props.id)}</title>
    </g>
  )
}
