import { Component, createMemo, For, Match, Show, Switch } from 'solid-js'
import { ContractSection } from '../../contract/contract-section'
import { t } from '../../i18n'
import { Model, Unit } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { PathText } from '../../shell/path-text'
import { ViewProps } from '../view-def'
import { archPathOf, goToLevel } from './actions'
import { BoxLink, Boxes, boxesAt, linkOf } from './boxes'
import { crossSliceBundles } from './bundles'
import { ROOT_SEGMENT, segmentsOf } from './components'
import { isPortSelection, PortDetails, SystemDetails, SystemOutline } from './system-panels'
import { systemsIn, SystemsInModel } from './systems'
import { LinkDetails, selectedUnit, selectedUnitPair, Swatch, UnitsOutline } from './unit-panels'
import { unitDataOf, UnitLink } from './units'

const BUNDLE_LIST = 12
const NEIGHBOURS = 12

const byCount = (a: UnitLink, b: UnitLink): number => b.count - a.count || a.key.localeCompare(b.key)

const BundleRows: Component<{ model: Model; links: readonly UnitLink[]; testId?: string }> = (props) => (
  <div class="arch-rows">
    <For each={props.links}>
      {(l) => (
        <button
          class="arch-row"
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
      <p class="arch-note">{t('arch.containers.hint')}</p>
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
  const info = () => unitDataOf(props.model).units.get(props.unit.id)
  return (
    <div data-testid="arch-container-details" data-id={props.unit.id}>
      <div class="det-path">
        <PathText path={props.unit.path} />
      </div>
      <p class="arch-note">
        {t('arch.modules', { n: props.unit.modules.length })} ·{' '}
        {t('arch.segments', { n: segmentsOf(props.model, props.unit).length })}
      </p>
      <div class="arch-btn-row">
        <button
          class="arch-btn"
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

const BoxLinkCard: Component<{ model: Model; data: Boxes; link: BoxLink }> = (props) => {
  const name = (key: string) => props.data.boxes.find((b) => b.key === key)?.name ?? key
  const units = createMemo(() =>
    unitDataOf(props.model)
      .links.filter((l) => props.data.boxOfUnit.get(l.from) === props.link.from && props.data.boxOfUnit.get(l.to) === props.link.to)
      .slice()
      .sort(byCount),
  )
  return (
    <div data-testid="arch-link-details" data-from={props.link.from} data-to={props.link.to}>
      <div class="det-section">
        <h4>
          {name(props.link.from)} → {name(props.link.to)}
        </h4>
        <p class="arch-note">
          {t('arch.imports', { n: props.link.count })}
          <Show when={props.link.findings}>
            {' · '}
            <span class="c-err" data-ch="sev">
              {t('arch.box.findings', { n: props.link.findings })}
            </span>
          </Show>
        </p>
      </div>
      <ContractSection edges={units().map((l) => ({ level: 'container' as const, from: l.from, to: l.to }))} />
      <Show
        when={units().length === 1 && units()[0]}
        fallback={
          <div class="det-section">
            <h4>{t('arch.link.bundles')}</h4>
            <BundleRows model={props.model} links={units()} testId="arch-bundle-row" />
          </div>
        }
      >
        {(l) => <LinkDetails model={props.model} link={l()} />}
      </Show>
    </div>
  )
}

const ContainersDetails: Component<{ model: Model }> = (props) => {
  const data = createMemo(() => unitDataOf(props.model))
  const boxes = createMemo(() => boxesAt(props.model, archPathOf(props.model)))
  const link = createMemo(() => {
    const p = selectedUnitPair()
    return p ? (data().linkOf(p.from, p.to) ?? null) : null
  })
  const boxLink = createMemo(() => {
    const sel = nav().sel
    return sel?.kind === 'link' ? linkOf(boxes(), sel.from, sel.to) : null
  })
  const unit = createMemo(() => {
    const id = selectedUnit()
    return id === null ? null : (props.model.units.find((u) => u.id === id) ?? null)
  })
  const systems = createMemo(() => (isPortSelection() ? systemsIn(props.model) : null))
  return (
    <Switch fallback={<ContainersOverview model={props.model} />}>
      <Match when={systems()}>{(s) => <PortDetails model={props.model} systems={s()} boxes={boxes()} />}</Match>
      <Match when={boxLink()}>{(l) => <BoxLinkCard model={props.model} data={boxes()} link={l()} />}</Match>
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
  <div class="arch-rows">
    <For each={props.ids}>
      {(id) => (
        <button
          class="arch-row"
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
      <div class="arch-btn-row">
        <button
          class="arch-btn"
          data-testid="arch-open-code"
          onClick={() => goToLevel(props.model, { level: 'code', container: props.unit.id, module: props.id })}
        >
          {t('arch.openCode')} <kbd>Enter</kbd>
        </button>
      </div>
      <p class="arch-note">
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
    <p class="arch-note">{t('arch.module.hint')}</p>
    <div class="det-section">
      <h4>{t('arch.segments', { n: segmentsOf(props.model, props.unit).length })}</h4>
      <div class="arch-rows">
        <For each={segmentsOf(props.model, props.unit)}>
          {(s) => (
            <div class="arch-row">
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
  if (path.level === 'system' || path.level === 'containers' || path.level === 'group') return null
  return m.units.find((u) => u.id === path.container) ?? null
}

const systemsOfPath = (m: Model): SystemsInModel | null => (archPathOf(m).level === 'system' ? systemsIn(m) : null)

const SystemPanel: Component<{ model: Model; systems: SystemsInModel }> = (props) => (
  <div class="panel panel-details">
    <div class="panel-hd">
      <span class="panel-title">{t('arch.systems.title')}</span>
      <span class="panel-count">{t('arch.systems.count', { n: props.systems.others.length + 1 })}</span>
    </div>
    <div class="panel-body">
      <SystemDetails model={props.model} systems={props.systems} />
    </div>
  </div>
)

export const ArchDetails: Component<ViewProps> = (props) => {
  const unit = createMemo(() => unitOfPath(props.model))
  const systems = createMemo(() => systemsOfPath(props.model))
  return (
    <Show when={systems()} fallback={<ContainerPanel model={props.model} unit={unit()} />}>
      {(s) => <SystemPanel model={props.model} systems={s()} />}
    </Show>
  )
}

const ContainerPanel: Component<{ model: Model; unit: Unit | null }> = (props) => (
  <div class="panel panel-details">
    <div class="panel-hd">
      <span class="panel-title">
        <Show when={props.unit} fallback={t('arch.containers.title')}>
          {(u) => props.model.unitLabel(u().id)}
        </Show>
      </span>
      <span class="panel-count">
        <Show when={props.unit} fallback={t('arch.units', { n: props.model.units.length })}>
          {(u) => t('arch.modules', { n: u().modules.length })}
        </Show>
      </span>
    </div>
    <div class="panel-body">
      <Show when={props.unit} fallback={<ContainersDetails model={props.model} />}>
        {(u) => <ComponentsDetails model={props.model} unit={u()} />}
      </Show>
    </div>
  </div>
)

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
              <div class="arch-ol-layer">
                <Swatch model={props.model} layer={props.unit.layer} />
                {s.name === ROOT_SEGMENT ? t('arch.root') : `${s.name}/`}
              </div>
              <For each={s.modules}>
                {(id) => (
                  <div
                    class={`arch-ol-unit${selected() === id ? ' on' : ''}`}
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
  const systems = createMemo(() => systemsOfPath(props.model))
  return (
    <Switch fallback={<UnitsOutline {...props} />}>
      <Match when={systems()}>{(s) => <SystemOutline systems={s()} />}</Match>
      <Match when={unit()}>{(u) => <ComponentsOutline model={props.model} unit={u()} />}</Match>
    </Switch>
  )
}
