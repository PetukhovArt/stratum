import { Component, createMemo, Match, onCleanup, onMount, Switch } from 'solid-js'
import { Model } from '../../render/model'
import { isTypingTarget } from '../../shell/typing'
import { ViewDef, ViewProps } from '../view-def'
import { archPathOf, goDown, goUp } from './actions'
import './arch.css'
import { boxesAt } from './boxes'
import { BoxesScene } from './boxes-scene'
import { ComponentsScene } from './components-scene'
import { C4Caption, Crumbs } from './crumbs'
import { ArchOutline, ArchDetails } from './panels'
import { SystemScene } from './system-scene'
import { systemsIn } from './systems'

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

const GroupLevel: Component<{ model: Model; group: string; name: string }> = (props) => {
  const data = createMemo(() => boxesAt(props.model, { level: 'group', group: props.group, name: props.name }))
  return (
    <BoxesScene model={props.model} data={data()} parent={{ key: props.group, name: props.name }} fitKey={`group:${props.group}`} />
  )
}

const BoxesLevel: Component<{ model: Model }> = (props) => {
  const data = createMemo(() => boxesAt(props.model, { level: 'containers' }))
  return <BoxesScene model={props.model} data={data()} fitKey="arch-boxes" />
}

const Stage: Component<ViewProps> = (props) => {
  const path = createMemo(() => archPathOf(props.model))
  const group = createMemo(() => {
    const p = path()
    return p.level === 'group' ? p : null
  })
  const unit = createMemo(() => {
    const p = path()
    return p.level === 'components' || p.level === 'code'
      ? (props.model.units.find((u) => u.id === p.container) ?? null)
      : null
  })
  const systems = createMemo(() => (path().level === 'system' ? systemsIn(props.model) : null))
  onMount(() => {
    const listener = (e: KeyboardEvent) => levelKey(e, props.model)
    window.addEventListener('keydown', listener)
    onCleanup(() => window.removeEventListener('keydown', listener))
  })
  return (
    <div class="arch-stage" data-testid="arch-stage" data-level={path().level}>
      <Crumbs model={props.model} path={path()} />
      <div class="arch-canvas">
        <C4Caption level={path().level} />
        <Switch fallback={<BoxesLevel model={props.model} />}>
          <Match when={systems()}>{(s) => <SystemScene model={props.model} systems={s()} />}</Match>
          <Match when={group()}>{(g) => <GroupLevel model={props.model} group={g().group} name={g().name} />}</Match>
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
