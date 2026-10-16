import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { t } from '../../i18n'
import { Model } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { layerName, layerTone, shortRule } from '../focus/paint'
import { ViewProps } from '../view-def'
import { selectedLayerPair, selectedUnit, selectedUnitPair, toggleExpanded } from './selection'
import { byFanIn, LayerLink, mapDataOf, MapData, UnitInfo, UnitLink } from './units'

const HOTSPOTS = 8
const NEIGHBOURS = 10
const BREAK_LIST = 12
const PAIR_LIST = 40

const unitName = (m: Model, id: number): string => m.containers.get(id)?.name ?? String(id)
const unitPath = (m: Model, id: number): string => m.containers.get(id)?.path ?? ''

const byCount = (a: UnitLink, b: UnitLink): number => b.count - a.count || a.key.localeCompare(b.key)

const Swatch: Component<{ model: Model; layer: number }> = (props) => (
  <span class="map-sw" style={{ background: layerTone(props.model, props.layer) }} />
)

const Stat: Component<{ value: number; label: string; isError?: boolean; testId?: string }> = (props) => (
  <div class={props.isError ? 'stat stat-err' : 'stat'}>
    <div class="stat-val" data-testid={props.testId} data-ch={props.isError ? 'sev' : undefined}>
      {props.value}
    </div>
    <div class="stat-lbl">{props.label}</div>
  </div>
)

const UnitRows: Component<{
  model: Model
  units: readonly UnitInfo[]
  value: (u: UnitInfo) => string
  testId: string
}> = (props) => (
  <div class="map-rows" data-testid={props.testId}>
    <For each={props.units}>
      {(u) => (
        <button
          class="map-row"
          data-id={u.unit.id}
          title={u.unit.path}
          onClick={() => set({ sel: { kind: 'container', id: u.unit.id } })}
        >
          <Swatch model={props.model} layer={u.unit.layer} />
          <span class="nm">{u.unit.name}</span>
          <span class="n">{props.value(u)}</span>
        </button>
      )}
    </For>
  </div>
)

const LinkRows: Component<{ model: Model; links: readonly UnitLink[]; side: 'from' | 'to' }> = (props) => (
  <div class="map-rows">
    <For each={props.links}>
      {(l) => {
        const other = props.side === 'from' ? l.from : l.to
        return (
          <button
            class="map-row"
            title={unitPath(props.model, other)}
            onClick={() => set({ sel: { kind: 'cell', level: 'unit', from: l.from, to: l.to } })}
          >
            <Swatch model={props.model} layer={props.model.containers.get(other)?.layer ?? 0} />
            <span class="nm">{unitName(props.model, other)}</span>
            <span class="n">{l.count}</span>
            <Show when={l.breaks}>
              <span class="n c-err" data-ch="sev">
                {l.breaks}
              </span>
            </Show>
          </button>
        )
      }}
    </For>
  </div>
)

const ruleCounts = (info: UnitInfo): [string, number][] => {
  const counts = new Map<string, number>()
  for (const v of info.findings) counts.set(v.rule, (counts.get(v.rule) ?? 0) + 1)
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
}

const WholeGraph: Component<{ model: Model; data: MapData }> = (props) => {
  const infos = () => [...props.data.units.values()]
  const hot = createMemo(() => infos().slice().sort(byFanIn).slice(0, HOTSPOTS))
  const erring = createMemo(() =>
    infos()
      .filter((u) => u.errors > 0)
      .sort((a, b) => b.errors - a.errors || a.unit.id - b.unit.id)
      .slice(0, HOTSPOTS),
  )
  const breaking = createMemo(() =>
    props.data.links
      .filter((l) => l.breaks > 0)
      .slice()
      .sort(byCount)
      .slice(0, BREAK_LIST),
  )
  return (
    <>
      <div class="det-stats det-stats-grid" data-testid="map-stats">
        <Stat value={props.data.links.length} label={t('map.stat.unitEdges', { n: props.data.links.length })} />
        <Stat
          value={props.data.breakLinks}
          label={t('map.stat.breakEdges', { n: props.data.breakLinks })}
          isError
          testId="map-break-links"
        />
        <Stat value={props.model.unitSccs[0]?.length ?? 0} label={t('map.stat.cycle')} />
      </div>
      <div class="det-section">
        <h4>{t('map.hot.fanIn')}</h4>
        <UnitRows model={props.model} units={hot()} value={(u) => t('map.units', { n: u.fanIn })} testId="map-hotspots" />
      </div>
      <Show when={erring().length}>
        <div class="det-section">
          <h4>{t('map.hot.errors')}</h4>
          <UnitRows model={props.model} units={erring()} value={(u) => String(u.errors)} testId="map-error-hotspots" />
        </div>
      </Show>
      <Show when={breaking().length}>
        <div class="det-section">
          <h4>{t('map.breakLinks')}</h4>
          <div class="map-rows">
            <For each={breaking()}>
              {(l) => (
                <button
                  class="map-row"
                  onClick={() => set({ sel: { kind: 'cell', level: 'unit', from: l.from, to: l.to } })}
                >
                  <span class="nm mono">
                    {unitName(props.model, l.from)} → {unitName(props.model, l.to)}
                  </span>
                  <span class="n">{l.count}</span>
                  <span class="n c-err" data-ch="sev">
                    {l.breaks}
                  </span>
                </button>
              )}
            </For>
          </div>
        </div>
      </Show>
    </>
  )
}

const UnitDetails: Component<{ model: Model; info: UnitInfo }> = (props) => {
  const u = () => props.info.unit
  const entry = () => props.model.containers.get(u().id)?.entry ?? null
  const isExpanded = () => nav().mapExpanded.includes(u().id)
  return (
    <div data-testid="map-unit-details">
      <div class="det-path">{u().path}</div>
      <div class="map-btn-row">
        <button class="map-btn" data-testid="map-expand" onClick={() => toggleExpanded(u().id)}>
          {isExpanded() ? t('map.collapse') : t('map.expand')}
        </button>
        <Show when={entry()}>
          {(id) => (
            <button class="map-btn" onClick={() => go({ view: 'focus', focus: id() })}>
              {t('map.focusEntry')}
            </button>
          )}
        </Show>
        <button class="map-btn" onClick={() => go({ view: 'dsm', dsmLevel: 'unit' })}>
          {t('map.findInMatrix')}
        </button>
      </div>
      <div class="det-stats det-stats-grid">
        <Stat value={u().modules.length} label={t('map.stat.modules', { n: u().modules.length })} />
        <Stat value={props.info.fanIn} label={t('map.stat.in')} />
        <Stat value={props.info.fanOut} label={t('map.stat.out')} />
      </div>
      <Show when={props.info.inn.length}>
        <div class="det-section">
          <h4>{t('map.importedBy')}</h4>
          <LinkRows model={props.model} links={props.info.inn.slice().sort(byCount).slice(0, NEIGHBOURS)} side="from" />
        </div>
      </Show>
      <Show when={props.info.out.length}>
        <div class="det-section">
          <h4>{t('map.importsOf')}</h4>
          <LinkRows model={props.model} links={props.info.out.slice().sort(byCount).slice(0, NEIGHBOURS)} side="to" />
        </div>
      </Show>
      <Show when={props.info.findings.length}>
        <div class="det-section">
          <h4>{t('map.findings', { n: props.info.findings.length })}</h4>
          <div class="map-rows">
            <For each={ruleCounts(props.info)}>
              {([rule, n]) => (
                <div class="map-row">
                  <span class="nm mono">{shortRule(rule)}</span>
                  <span class="n">{n}</span>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  )
}

const LinkDetails: Component<{ model: Model; link: UnitLink }> = (props) => {
  const pairs = createMemo(() =>
    props.model.edges
      .filter((e) => props.model.unitOf(e.from) === props.link.from && props.model.unitOf(e.to) === props.link.to)
      .map((e) => ({ ...e, findings: props.model.findingsOnEdge(e.from, e.to) }))
      .sort((a, b) => b.findings.length - a.findings.length || a.from - b.from || a.to - b.to),
  )
  return (
    <div class="det-section" data-testid="map-link">
      <h4>
        {unitName(props.model, props.link.from)} → {unitName(props.model, props.link.to)}
      </h4>
      <div class="det-path">
        {unitPath(props.model, props.link.from)}
        <br />→ {unitPath(props.model, props.link.to)}
      </div>
      <p class="map-note">
        {t('map.imports', { n: props.link.count })}
        <Show when={props.link.breaks}>
          {' · '}
          <span class="c-err" data-ch="sev">
            {t('map.breaks', { n: props.link.breaks })}
          </span>
        </Show>
      </p>
      <Show when={props.link.rules.length}>
        <div class="map-rules mono">{props.link.rules.map(shortRule).join(', ')}</div>
      </Show>
      <h4>{t('map.pairs')}</h4>
      <div class="map-rows">
        <For each={pairs().slice(0, PAIR_LIST)}>
          {(e) => {
            const sev = props.model.worst(e.findings)
            return (
              <button
                class="map-row"
                title={`${props.model.modules.get(e.from)?.path}\n→ ${props.model.modules.get(e.to)?.path}`}
                onClick={() => go({ view: 'focus', focus: e.from })}
              >
                <span class="nm mono">
                  {props.model.label(e.from)} → {props.model.label(e.to)}
                </span>
                <Show when={sev}>{(s) => <span class={`map-dot sev-${s()}`} data-ch="sev" />}</Show>
              </button>
            )
          }}
        </For>
        <Show when={pairs().length > PAIR_LIST}>
          <div class="empty">{t('map.more', { n: pairs().length - PAIR_LIST })}</div>
        </Show>
      </div>
    </div>
  )
}

const LayerLinkDetails: Component<{ model: Model; link: LayerLink }> = (props) => (
  <div class="det-section" data-testid="map-layer-link">
    <h4>
      {layerName(props.model, props.link.from)} → {layerName(props.model, props.link.to)}
    </h4>
    <p class="map-note">
      {t('map.imports', { n: props.link.count })} {t('map.unitPairs', { n: props.link.unitPairs })} ·{' '}
      {props.link.isDeclared ? t('map.declared') : t('map.undeclared')}
    </p>
    <Show when={props.link.rules.length}>
      <div class="map-rules mono">{props.link.rules.map(shortRule).join(', ')}</div>
    </Show>
  </div>
)

export const MapDetails: Component<ViewProps> = (props) => {
  const data = createMemo(() => mapDataOf(props.model))
  const unit = createMemo(() => {
    const id = selectedUnit()
    return id === null ? null : (data().units.get(id) ?? null)
  })
  const link = createMemo(() => {
    const p = selectedUnitPair()
    return p ? (data().linkOf(p.from, p.to) ?? null) : null
  })
  const layerLink = createMemo(() => {
    const p = selectedLayerPair()
    return p ? (data().layerLinkOf(p.from, p.to) ?? null) : null
  })
  return (
    <div class="panel panel-details">
      <div class="panel-hd">
        <span class="panel-title">{unit()?.unit.name ?? t('map.whole')}</span>
        <span class="panel-count">{unit()?.unit.kind ?? t('map.units', { n: props.model.units.length })}</span>
      </div>
      <div class="panel-body">
        <Show when={link()}>{(l) => <LinkDetails model={props.model} link={l()} />}</Show>
        <Show when={layerLink()}>{(l) => <LayerLinkDetails model={props.model} link={l()} />}</Show>
        <Show when={unit()} fallback={<WholeGraph model={props.model} data={data()} />}>
          {(info) => <UnitDetails model={props.model} info={info()} />}
        </Show>
      </div>
    </div>
  )
}

export const MapOutline: Component<ViewProps> = (props) => {
  const [query, setQuery] = createSignal('')
  const data = createMemo(() => mapDataOf(props.model))
  const groups = createMemo(() => {
    const q = query().trim().toLowerCase()
    return props.model.layers
      .map((layer) => ({
        layer,
        units: props.model.units
          .filter((u) => u.layer === layer.id && (!q || u.path.toLowerCase().includes(q)))
          .sort((a, b) => b.modules.length - a.modules.length || a.id - b.id),
      }))
      .filter((g) => g.units.length)
  })
  return (
    <aside class="panel">
      <header class="panel-hd">
        <span class="panel-title">{t('map.outline')}</span>
        <span class="panel-count">{props.model.units.length}</span>
      </header>
      <input
        class="map-filter"
        placeholder={t('map.filter')}
        value={query()}
        onInput={(e) => setQuery(e.currentTarget.value)}
      />
      <div class="outline-tree">
        <For each={groups()}>
          {(g) => (
            <>
              <div class="map-ol-layer">
                <Swatch model={props.model} layer={g.layer.id} />
                {g.layer.name}
              </div>
              <For each={g.units}>
                {(u) => (
                  <div
                    class={`map-ol-unit${selectedUnit() === u.id ? ' on' : ''}`}
                    data-testid="map-outline-row"
                    title={u.path}
                    onClick={() => set({ sel: { kind: 'container', id: u.id } })}
                  >
                    <span class="nm">{u.name}</span>
                    <Show when={(data().units.get(u.id)?.errors ?? 0) > 0}>
                      <span class="map-dot sev-error" data-ch="sev" />
                    </Show>
                    <span class="n">{u.modules.length}</span>
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
