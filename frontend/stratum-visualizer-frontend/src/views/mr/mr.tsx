import { Component, JSX, Match, Show, Switch } from 'solid-js'
import { t } from '../../i18n'
import { Model } from '../../render/model'
import { nav } from '../../shell/nav'
import { ViewDef, ViewProps } from '../view-def'
import { CardScene } from './card'
import { DriftReport, driftReport } from './drift'
import './mr.css'
import { MrDetails, MrOutline } from './panels'
import { Review, toReview } from './review'
import { RegionScene } from './scene'

const reviews = new WeakMap<DriftReport, WeakMap<Model, Review>>()

const reviewOf = (m: Model, drift: DriftReport): Review => {
  const byModel = reviews.get(drift) ?? new WeakMap<Model, Review>()
  reviews.set(drift, byModel)
  const cached = byModel.get(m)
  if (cached) return cached
  const made = toReview(m, drift)
  byModel.set(m, made)
  return made
}

const EmptyPanel: Component = () => <div class="panel" />

const WithDrift: Component<{
  model: Model
  fallback?: JSX.Element
  children: (drift: DriftReport, review: Review) => JSX.Element
}> = (props) => (
  <Show when={driftReport()} fallback={props.fallback ?? <EmptyPanel />}>
    {(drift) => props.children(drift(), reviewOf(props.model, drift()))}
  </Show>
)

const Stage: Component<ViewProps> = (props) => (
  <WithDrift
    model={props.model}
    fallback={
      <div class="placeholder" data-testid="mr-unavailable">
        <h3>{t('view.mr')}</h3>
        <p>{t('mr.unavailable')}</p>
      </div>
    }
  >
    {(drift, review) => (
      <div class="mr-stage" data-testid="mr-stage">
        <Switch>
          <Match when={nav().mrVariant === 'region'}>
            <RegionScene model={props.model} review={review} />
          </Match>
          <Match when={nav().mrVariant === 'card'}>
            <CardScene model={props.model} review={review} drift={drift} />
          </Match>
        </Switch>
      </div>
    )}
  </WithDrift>
)

const Outline: Component<ViewProps> = (props) => (
  <WithDrift model={props.model}>{(_, review) => <MrOutline review={review} />}</WithDrift>
)

const Details: Component<ViewProps> = (props) => (
  <WithDrift model={props.model}>
    {(drift, review) => <MrDetails model={props.model} review={review} drift={drift} />}
  </WithDrift>
)

export const mrView: ViewDef = {
  id: 'mr',
  key: '3',
  title: 'view.mr',
  Stage,
  Outline,
  Details,
  isAvailable: () => driftReport() !== null,
  variants: {
    key: 'mrVariant',
    options: [
      { id: 'region', title: 'mr.variant.region' },
      { id: 'card', title: 'mr.variant.card' },
    ],
  },
}
