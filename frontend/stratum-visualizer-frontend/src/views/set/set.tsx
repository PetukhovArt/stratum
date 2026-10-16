import { Component, Show } from 'solid-js'
import { t } from '../../i18n'
import { Model } from '../../render/model'
import { nav } from '../../shell/nav'
import { ViewDef, ViewProps } from '../view-def'
import { analyzeSet, SetAnalysis } from './analysis'
import { SetDetails, SetOutline } from './panels'
import { SetScene } from './scene'
import './set.css'

let last: { model: Model; key: string; analysis: SetAnalysis } | null = null

const currentAnalysis = (m: Model): SetAnalysis => {
  const ids = nav().set
  const key = ids.join(',')
  if (last?.model === m && last.key === key) return last.analysis
  last = { model: m, key, analysis: analyzeSet(m, ids) }
  return last.analysis
}

const Bar: Component<{ analysis: SetAnalysis }> = (props) => (
  <div class="set-bar" data-testid="set-bar">
    <span>{t('set.bar.members', { n: props.analysis.members.length })}</span>
    <Show when={props.analysis.via.length}>
      <span>{t('set.bar.via', { n: props.analysis.via.length })}</span>
    </Show>
    <span>
      {props.analysis.loop ? t('set.bar.loop', { n: props.analysis.loop.length }) : t('set.bar.noLoop')}
    </span>
  </div>
)

const Stage: Component<ViewProps> = (props) => {
  const analysis = () => currentAnalysis(props.model)
  return (
    <div class="set-stage" data-testid="set-stage">
      <Show
        when={analysis().members.length}
        fallback={
          <div class="set-empty" data-testid="set-empty">
            {t('set.empty')}
          </div>
        }
      >
        <SetScene model={props.model} analysis={analysis()} />
        <Bar analysis={analysis()} />
      </Show>
    </div>
  )
}

const Details: Component<ViewProps> = (props) => (
  <SetDetails model={props.model} analysis={currentAnalysis(props.model)} />
)

const Outline: Component<ViewProps> = (props) => <SetOutline model={props.model} />

export const setView: ViewDef = {
  id: 'set',
  key: '5',
  title: 'view.set',
  Stage,
  Details,
  Outline,
  variants: {
    key: 'setVariant',
    options: [
      { id: 'ring', title: 'set.variant.ring' },
      { id: 'arc', title: 'set.variant.arc' },
      { id: 'matrix', title: 'set.variant.matrix' },
    ],
  },
}
