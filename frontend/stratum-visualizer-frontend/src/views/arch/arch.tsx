import { Component, createMemo, Match, onCleanup, onMount, Switch } from 'solid-js'
import { Model } from '../../render/model'
import { nav } from '../../shell/nav'
import { isTypingTarget } from '../../shell/typing'
import { GateBar } from '../map/gate-bar'
import { LanesScene } from '../map/lanes-scene'
import '../map/map.css'
import { mapDataOf } from '../map/units'
import { ViewDef, ViewProps } from '../view-def'
import { archPathOf, goDown, goToLevel, goUp } from './actions'
import './arch.css'
import { ComponentsScene } from './components-scene'
import { Crumbs } from './crumbs'
import { ArchOutline, ArchDetails } from './panels'

const MARKS = { unit: 'arch-container', bundle: 'arch-bundle' } as const
const NO_EXPANDED: readonly number[] = []

const isControl = (target: EventTarget | null): boolean =>
  target instanceof Element && target.closest('button, a, select') !== null

const levelKey = (e: KeyboardEvent, m: Model): void => {
  if (e.metaKey || e.ctrlKey || e.altKey || isTypingTarget(e.target)) return
  if (e.key === 'Enter' && !isControl(e.target) && goDown(m)) {
    e.preventDefault()
  } else if (e.key === 'Backspace' && goUp(m)) {
    e.preventDefault()
  }
}

const ContainersLevel: Component<{ model: Model }> = (props) => (
  <>
    <LanesScene
      model={props.model}
      gate={nav().mapGate}
      expanded={NO_EXPANDED}
      onUnitDblClick={(id) => goToLevel(props.model, { level: 'components', container: id })}
      marks={MARKS}
    />
    <GateBar total={mapDataOf(props.model).links.length} />
  </>
)

const Stage: Component<ViewProps> = (props) => {
  const path = createMemo(() => archPathOf(props.model))
  const unit = createMemo(() => {
    const p = path()
    return p.level === 'containers' ? null : (props.model.units.find((u) => u.id === p.container) ?? null)
  })
  onMount(() => {
    const listener = (e: KeyboardEvent) => levelKey(e, props.model)
    window.addEventListener('keydown', listener)
    onCleanup(() => window.removeEventListener('keydown', listener))
  })
  return (
    <div class="arch-stage" data-testid="arch-stage" data-level={path().level}>
      <Crumbs model={props.model} path={path()} />
      <div class="arch-canvas">
        <Switch fallback={<ContainersLevel model={props.model} />}>
          <Match when={unit()}>{(u) => <ComponentsScene model={props.model} unit={u()} />}</Match>
        </Switch>
      </div>
    </div>
  )
}

export const archView: ViewDef = {
  id: 'arch',
  key: '6',
  title: 'view.arch',
  Stage,
  Outline: ArchOutline,
  Details: ArchDetails,
}
