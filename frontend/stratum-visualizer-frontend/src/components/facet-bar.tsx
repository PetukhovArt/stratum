import { Component, For, Show } from 'solid-js'
import { activeFilterCount, clearFilters, toggled } from '../lib/filters'
import { DesignData, EdgeKindLabel } from '../render/design'
import { purityInfo, RankInfo, RANKS, stageInfo } from '../render/palette'
import { ALL_EDGE_KINDS, COLOR_MODES, ColorMode, Filters } from '../state'

const EDGE_KIND_TOOLTIP: Record<EdgeKindLabel, string> = {
  static: 'Static imports — resolved at build time (import / export).',
  di: 'DI edges — injected at runtime through your container/provider.',
  runtime: 'Runtime edges — lazy / dynamic imports, conditional loads.',
}

type RankFilterKey = 'stageFilter' | 'purityFilter'

interface RankAxis {
  key: RankFilterKey
  label: string
  scope: string
  info: (rank: number) => RankInfo
}

const RANK_AXES: readonly RankAxis[] = [
  { key: 'stageFilter', label: 'Stage', scope: 'containers', info: stageInfo },
  { key: 'purityFilter', label: 'Purity', scope: 'modules', info: purityInfo },
]

const toggleRank = (f: Filters, key: RankFilterKey, rank: number): Filters => ({
  ...f,
  [key]: f[key] === rank ? null : rank,
})

interface FacetBarProps {
  filters: Filters
  setFilters: (f: Filters) => void
  colorMode: ColorMode
  setColorMode: (m: ColorMode) => void
  isHaloThick: boolean
  onHaloToggle: () => void
  data: DesignData
}

export const FacetBar: Component<FacetBarProps> = (props) => {
  const toggleLayer = (id: string) =>
    props.setFilters({ ...props.filters, disabledLayers: toggled(props.filters.disabledLayers, id) })
  const toggleKind = (k: EdgeKindLabel) =>
    props.setFilters({ ...props.filters, edgeKinds: toggled(props.filters.edgeKinds, k) })
  const hasActiveFilters = () => activeFilterCount(props.filters) > 0
  const clearAll = () => props.setFilters(clearFilters(props.filters))

  return (
    <div class="facet-bar">
      <div class="facet-row">
        <div class="facet-group">
          <span class="facet-label">Layers</span>
          <For each={props.data.layers}>
            {(l) => {
              const off = () => props.filters.disabledLayers.has(l.id)
              const tip = `${l.label} — ${l.modules} modules${
                l.errors ? `, ${l.errors} errors` : ''
              }${l.warnings ? `, ${l.warnings} warnings` : ''}.\nClick to toggle visibility.`
              return (
                <button
                  class={`chip chip-layer ${off() ? 'chip-off' : ''}`}
                  onClick={() => toggleLayer(l.id)}
                  style={{ ['--hue' as string]: l.hue }}
                  title={tip}
                >
                  <span class="chip-dot" />
                  <span class="chip-name">{l.label}</span>
                  <span class="chip-num">{l.modules}</span>
                </button>
              )
            }}
          </For>
        </div>
        <span class="facet-sep" />
        <div class="facet-group">
          <span class="facet-label">Edges</span>
          <For each={ALL_EDGE_KINDS}>
            {(k) => {
              const off = () => !props.filters.edgeKinds.has(k)
              return (
                <button
                  class={`chip chip-edge chip-edge-${k} ${off() ? 'chip-off' : ''}`}
                  onClick={() => toggleKind(k)}
                  title={EDGE_KIND_TOOLTIP[k]}
                >
                  <span class="chip-stroke" data-kind={k} />
                  <span class="chip-name">{k}</span>
                </button>
              )
            }}
          </For>
        </div>
        <span class="facet-sep" />
        <div class="facet-group">
          <button
            class={`chip chip-mode ${props.filters.isViolatorsOnly ? 'on' : ''}`}
            onClick={() =>
              props.setFilters({ ...props.filters, isViolatorsOnly: !props.filters.isViolatorsOnly })
            }
            title="Show only modules that have violations (or descendants with violations)."
          >
            <span class="chip-dot sev-err" />
            <span class="chip-name">Only violators</span>
          </button>
          <button
            class={`chip chip-mode ${props.filters.connectionMode === 'all' ? 'on' : ''}`}
            onClick={() =>
              props.setFilters({
                ...props.filters,
                connectionMode: props.filters.connectionMode === 'all' ? 'minimal' : 'all',
              })
            }
            title="Paint every edge in the graph. Off by default — on real projects this draws thousands of lines and saturates the canvas. Hover a module to see its edges either way."
          >
            <span class="chip-stroke" data-kind="static" />
            <span class="chip-name">Show all edges</span>
          </button>
        </div>
        <span class="facet-sep" />
        <div class="facet-group facet-glob">
          <span class="facet-label">Path</span>
          <input
            class="glob-input"
            placeholder="auth , src/billing/** , !**/legacy/**"
            value={props.filters.glob}
            onInput={(e) => props.setFilters({ ...props.filters, glob: e.currentTarget.value })}
            title="Path filter. Bare words like `gis` match as case-insensitive substring. Globs (`*`, `**`, `?`) match path segments. Comma-separated. Prefix with `!` to exclude."
          />
          <Show when={props.filters.glob !== ''}>
            <button
              class="glob-clear"
              onClick={() => props.setFilters({ ...props.filters, glob: '' })}
            >
              ✕
            </button>
          </Show>
        </div>
        <Show when={hasActiveFilters()}>
          <button class="facet-clear" onClick={clearAll}>
            clear all
          </button>
        </Show>
      </div>
      <div class="facet-row">
        <For each={RANK_AXES}>
          {(axis) => (
            <>
              <div class="facet-group" data-testid={`facet-${axis.key}`}>
                <span class="facet-label" title={`${axis.label} of ${axis.scope}`}>
                  {axis.label}
                </span>
                <For each={RANKS}>
                  {(rank) => (
                    <button
                      class={`chip chip-mode ${props.filters[axis.key] === rank ? 'on' : ''}`}
                      onClick={() => props.setFilters(toggleRank(props.filters, axis.key, rank))}
                      title={`${axis.info(rank).desc}\nShows ${axis.scope} at ${axis.label.toLowerCase()} ${rank}.`}
                      aria-label={`${axis.label} ${rank} (${axis.info(rank).name})`}
                      aria-pressed={props.filters[axis.key] === rank}
                    >
                      <span class="chip-dot" style={{ background: axis.info(rank).color }} />
                      <span class="chip-name">{rank}</span>
                    </button>
                  )}
                </For>
              </div>
              <span class="facet-sep" />
            </>
          )}
        </For>
        <div class="facet-group" data-testid="facet-color-mode">
          <span class="facet-label">Colour</span>
          <For each={COLOR_MODES}>
            {(mode) => (
              <button
                class={`chip chip-mode ${props.colorMode === mode ? 'on' : ''}`}
                onClick={() => props.setColorMode(mode)}
                title={`Colour modules by ${mode}.`}
                aria-label={`Colour by ${mode}`}
                aria-pressed={props.colorMode === mode}
              >
                <span class="chip-name">{mode}</span>
              </button>
            )}
          </For>
        </div>
        <span class="facet-sep" />
        <button
          class={`chip chip-mode ${props.isHaloThick ? 'on' : ''}`}
          onClick={props.onHaloToggle}
          title="Toggle compound halos"
          aria-pressed={props.isHaloThick}
        >
          <span class="chip-name">halos</span>
        </button>
      </div>
    </div>
  )
}
