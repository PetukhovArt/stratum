import { Component, createMemo, For, Match, Show, Switch } from 'solid-js'
import { ContractSection } from '../../contract/contract-section'
import { t } from '../../i18n'
import { Model, Unit } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { PathText } from '../../shell/path-text'
import { LinkDetails, MapOutline, Swatch } from '../map/panels'
import { selectedUnit, selectedUnitPair } from '../map/selection'
import { mapDataOf, UnitLink } from '../map/units'
import { ViewProps } from '../view-def'
import { archPathOf, goToLevel } from './actions'
import { crossSliceBundles } from './bundles'
import { ROOT_SEGMENT, segmentsOf } from './components'

const BUNDLE_LIST = 12
const NEIGHBOURS = 12

const byCount = (a: UnitLink, b: UnitLink): number => b.count - a.count || a.key.localeCompare(b.key)

const BundleRows: Component<{ model: Model; links: readonly UnitLink[]; testId?: string }> = (props) => (
  <div class="map-rows">
    <For each={props.links}>
      {(l) => (
        <button
          class="map-row"
          data-testid={props.testId}
          data-from={l.from}
          data-to={l.to}
          onClick={() => set({ sel: { kind: 'cell', level: 'unit', from: l.from, to: l.to } })}
        >
          <span class="nm mono">
            {props.model.unitLabel(l.from)} → {props.model.unitLabel(l.to)}
          </span>
          <span class="n">{l.count}</span>
          <Show when={l.breaks}>
            <span class="n c-err" data-ch="sev">
              {l.breaks}
            </span>
          </Show>
        </button>
      )}
    </For>
  </div>
)

const ContainersOverview: Component<{ model: Model }> = (props) => {
  const bundles = createMemo(() => crossSliceBundles(props.model))
  return (
    <>
      <p class="map-note">{t('arch.containers.hint')}</p>
      <div class="det-section">
        <h4>{t('arch.crossSlice')}</h4>
        <Show when={bundles().length} fallback={<div class="empty">{t('arch.noCrossSlice')}</div>}>
          <BundleRows model={props.model} links={bundles().slice(0, BUNDLE_LIST)} testId="arch-bundle-row" />
        </Show>
      </div>
    </>
  )
}

const ContainerCard: Component<{ model: Model; unit: Unit }> = (props) => {
  const info = () => mapDataOf(props.model).units.get(props.unit.id)
  return (
    <div data-testid="arch-container-details" data-id={props.unit.id}>
      <div class="det-path">
        <PathText path={props.unit.path} />
      </div>
      <p class="map-note">
        {t('arch.modules', { n: props.unit.modules.length })} ·{' '}
        {t('arch.segments', { n: segmentsOf(props.model, props.unit).length })}
      </p>
      <div class="map-btn-row">
        <button
          class="map-btn"
          data-testid="arch-enter"
          onClick={() => goToLevel(props.model, { level: 'components', container: props.unit.id })}
        >
          {t('arch.enter')} <kbd>Enter</kbd>
        </button>
      </div>
      <Show when={info()?.out.length}>
        <div class="det-section">
          <h4>{t('arch.bundlesOut')}</h4>
          <BundleRows model={props.model} links={(info()?.out ?? []).slice().sort(byCount).slice(0, NEIGHBOURS)} />
        </div>
      </Show>
      <Show when={info()?.inn.length}>
        <div class="det-section">
          <h4>{t('arch.bundlesIn')}</h4>
          <BundleRows model={props.model} links={(info()?.inn ?? []).slice().sort(byCount).slice(0, NEIGHBOURS)} />
        </div>
      </Show>
    </div>
  )
}

const ContainersDetails: Component<{ model: Model }> = (props) => {
  const data = createMemo(() => mapDataOf(props.model))
  const link = createMemo(() => {
    const p = selectedUnitPair()
    return p ? (data().linkOf(p.from, p.to) ?? null) : null
  })
  const unit = createMemo(() => {
    const id = selectedUnit()
    return id === null ? null : (props.model.units.find((u) => u.id === id) ?? null)
  })
  return (
    <Switch fallback={<ContainersOverview model={props.model} />}>
      <Match when={link()}>
        {(l) => (
          <div data-testid="arch-bundle-details" data-from={l().from} data-to={l().to}>
            <ContractSection edges={[{ level: 'container', from: l().from, to: l().to }]} />
            <LinkDetails model={props.model} link={l()} />
          </div>
        )}
      </Match>
      <Match when={unit()}>{(u) => <ContainerCard model={props.model} unit={u()} />}</Match>
    </Switch>
  )
}

const ModuleRows: Component<{ model: Model; ids: readonly number[] }> = (props) => (
  <div class="map-rows">
    <For each={props.ids}>
      {(id) => (
        <button
          class="map-row"
          title={props.model.modules.get(id)?.path}
          onClick={() => set({ sel: { kind: 'module', id } })}
        >
          <span class="nm mono">{props.model.label(id)}</span>
        </button>
      )}
    </For>
  </div>
)

const ModuleCard: Component<{ model: Model; unit: Unit; id: number }> = (props) => {
  const isInside = (other: number) => props.model.unitOf(other) === props.unit.id
  const out = () => props.model.out(props.id)
  const inn = () => props.model.inn(props.id)
  return (
    <div data-testid="arch-module-details" data-id={props.id}>
      <div class="det-path">
        <PathText path={props.model.modules.get(props.id)?.path ?? ''} />
      </div>
      <div class="map-btn-row">
        <button
          class="map-btn"
          data-testid="arch-open-code"
          onClick={() => goToLevel(props.model, { level: 'code', container: props.unit.id, module: props.id })}
        >
          {t('arch.openCode')} <kbd>Enter</kbd>
        </button>
      </div>
      <p class="map-note">
        {t('arch.outside', { out: out().filter((x) => !isInside(x)).length, inn: inn().filter((x) => !isInside(x)).length })}
      </p>
      <Show when={out().some(isInside)}>
        <div class="det-section">
          <h4>{t('arch.importsInside')}</h4>
          <ModuleRows model={props.model} ids={out().filter(isInside).slice(0, NEIGHBOURS)} />
        </div>
      </Show>
      <Show when={inn().some(isInside)}>
        <div class="det-section">
          <h4>{t('arch.importedInside')}</h4>
          <ModuleRows model={props.model} ids={inn().filter(isInside).slice(0, NEIGHBOURS)} />
        </div>
      </Show>
    </div>
  )
}

const ImportCard: Component<{ model: Model; from: number; to: number }> = (props) => (
  <div data-testid="arch-import-details" data-from={props.from} data-to={props.to}>
    <div class="det-section">
      <h4>
        {props.model.label(props.from)} → {props.model.label(props.to)}
      </h4>
      <div class="det-path">
        <PathText path={props.model.modules.get(props.from)?.path ?? ''} />
        <br />→ <PathText path={props.model.modules.get(props.to)?.path ?? ''} />
      </div>
    </div>
    <ContractSection edges={[{ level: 'module', from: props.from, to: props.to }]} />
  </div>
)

const ComponentsOverview: Component<{ model: Model; unit: Unit }> = (props) => (
  <>
    <div class="det-path">
      <PathText path={props.unit.path} />
    </div>
    <p class="map-note">{t('arch.module.hint')}</p>
    <div class="det-section">
      <h4>{t('arch.segments', { n: segmentsOf(props.model, props.unit).length })}</h4>
      <div class="map-rows">
        <For each={segmentsOf(props.model, props.unit)}>
          {(s) => (
            <div class="map-row">
              <span class="nm mono">{s.name === ROOT_SEGMENT ? t('arch.root') : `${s.name}/`}</span>
              <span class="n">{s.modules.length}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  </>
)

const ComponentsDetails: Component<{ model: Model; unit: Unit }> = (props) => {
  const sel = () => nav().sel
  const edge = () => {
    const s = sel()
    return s?.kind === 'edge' ? s : null
  }
  const module = () => {
    const s = sel()
    if (s?.kind === 'module' && props.model.unitOf(s.id) === props.unit.id) return s.id
    const path = archPathOf(props.model)
    return s === null && path.level === 'code' ? path.module : null
  }
  return (
    <Switch fallback={<ComponentsOverview model={props.model} unit={props.unit} />}>
      <Match when={edge()}>{(e) => <ImportCard model={props.model} from={e().from} to={e().to} />}</Match>
      <Match when={module()}>{(id) => <ModuleCard model={props.model} unit={props.unit} id={id()} />}</Match>
    </Switch>
  )
}

const unitOfPath = (m: Model): Unit | null => {
  const path = archPathOf(m)
  if (path.level === 'containers') return null
  return m.units.find((u) => u.id === path.container) ?? null
}

export const ArchDetails: Component<ViewProps> = (props) => {
  const unit = createMemo(() => unitOfPath(props.model))
  return (
    <div class="panel panel-details">
      <div class="panel-hd">
        <span class="panel-title">
          <Show when={unit()} fallback={t('arch.containers.title')}>
            {(u) => props.model.unitLabel(u().id)}
          </Show>
        </span>
        <span class="panel-count">
          <Show when={unit()} fallback={t('map.units', { n: props.model.units.length })}>
            {(u) => t('arch.modules', { n: u().modules.length })}
          </Show>
        </span>
      </div>
      <div class="panel-body">
        <Show when={unit()} fallback={<ContainersDetails model={props.model} />}>
          {(u) => <ComponentsDetails model={props.model} unit={u()} />}
        </Show>
      </div>
    </div>
  )
}

const ComponentsOutline: Component<{ model: Model; unit: Unit }> = (props) => {
  const selected = () => {
    const sel = nav().sel
    return sel?.kind === 'module' ? sel.id : null
  }
  return (
    <aside class="panel">
      <header class="panel-hd">
        <span class="panel-title">{props.model.unitLabel(props.unit.id)}</span>
        <span class="panel-count">{props.unit.modules.length}</span>
      </header>
      <div class="outline-tree">
        <For each={segmentsOf(props.model, props.unit)}>
          {(s) => (
            <>
              <div class="map-ol-layer">
                <Swatch model={props.model} layer={props.unit.layer} />
                {s.name === ROOT_SEGMENT ? t('arch.root') : `${s.name}/`}
              </div>
              <For each={s.modules}>
                {(id) => (
                  <div
                    class={`map-ol-unit${selected() === id ? ' on' : ''}`}
                    data-testid="arch-outline-module"
                    data-id={id}
                    title={props.model.modules.get(id)?.path}
                    onClick={() => set({ sel: { kind: 'module', id } })}
                    onDblClick={() => goToLevel(props.model, { level: 'code', container: props.unit.id, module: id })}
                  >
                    <span class="nm">{props.model.label(id)}</span>
                  </div>
                )}
              </For>
            </>
          )}
        </For>
      </div>
    </aside>
  )
}

export const ArchOutline: Component<ViewProps> = (props) => {
  const unit = createMemo(() => unitOfPath(props.model))
  return (
    <Show when={unit()} fallback={<MapOutline {...props} />}>
      {(u) => <ComponentsOutline model={props.model} unit={u()} />}
    </Show>
  )
}
