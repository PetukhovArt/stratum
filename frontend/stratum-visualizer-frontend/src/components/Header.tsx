import { Component, createMemo, For, Show } from 'solid-js'
import { activeFilterCount, clearFilters, toggled } from '../lib/filters'
import { normalizePath } from '../lib/glob'
import { DesignData, EdgeKindLabel } from '../render/design'
import { purityInfo, RankInfo, RANKS, stageInfo } from '../render/palette'
import { ALL_EDGE_KINDS, COLOR_MODES, ColorMode, Filters } from '../state'

const MAX_PALETTE_MATCHES = 20

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

interface HeaderProps {
  data: DesignData
  filters: Filters
  setFilters: (f: Filters) => void
  colorMode: ColorMode
  setColorMode: (m: ColorMode) => void
  isPaletteOpen: boolean
  setPaletteOpen: (v: boolean) => void
  onSelect: (id: string) => void
  onResetView: () => void
  onHaloToggle: () => void
}

export const Header: Component<HeaderProps> = (props) => {
  const counts = createMemo(() => {
    let err = 0
    let warn = 0
    for (const m of props.data.modules) {
      if (m.severity === 'error') err++
      else if (m.severity === 'warning') warn++
    }
    return { err, warn }
  })

  return (
    <header class="strat-hd">
      <div class="hd-l">
        <div class="brand">
          <span class="brand-mark" />
          <span class="brand-name">stratum</span>
        </div>
        <span class="hd-divider" />
        <span class="hd-project">{props.data.meta.project}</span>
      </div>

      <div class="hd-c">
        <CommandPalette
          isPaletteOpen={props.isPaletteOpen}
          setPaletteOpen={props.setPaletteOpen}
          filters={props.filters}
          setFilters={props.setFilters}
          colorMode={props.colorMode}
          setColorMode={props.setColorMode}
          data={props.data}
          onSelect={props.onSelect}
        />
      </div>

      <div class="hd-r">
        <div class="hd-stats">
          <span class="hd-stat">
            {props.data.modules.length} <em>mod</em>
          </span>
          <span class="hd-stat hd-stat-err">
            {counts().err} <em>err</em>
          </span>
          <span class="hd-stat hd-stat-warn">
            {counts().warn} <em>warn</em>
          </span>
        </div>
        <div class="hd-actions">
          <button class="hd-btn" onClick={props.onResetView} title="Fit to bounds (Esc)">
            ⤢ fit
          </button>
          <button class="hd-btn" onClick={props.onHaloToggle} title="Toggle compound halos">
            ⚙
          </button>
        </div>
      </div>

      <FacetBar
        filters={props.filters}
        setFilters={props.setFilters}
        colorMode={props.colorMode}
        setColorMode={props.setColorMode}
        data={props.data}
      />
    </header>
  )
}

interface PaletteProps {
  isPaletteOpen: boolean
  setPaletteOpen: (v: boolean) => void
  filters: Filters
  setFilters: (f: Filters) => void
  colorMode: ColorMode
  setColorMode: (m: ColorMode) => void
  data: DesignData
  onSelect: (id: string) => void
}

const CommandPalette: Component<PaletteProps> = (props) => {
  const query = () => props.filters.query
  const toggleKind = (kind: EdgeKindLabel) =>
    props.setFilters({ ...props.filters, edgeKinds: toggled(props.filters.edgeKinds, kind) })

  const rankCommands = (axis: RankAxis) => [
    ...RANKS.map((rank) => ({
      key: `${axis.key}-${rank}`,
      label: `Filter ${axis.scope} by ${axis.label.toLowerCase()}: ${rank} (${axis.info(rank).name})`,
      run: () => props.setFilters(toggleRank(props.filters, axis.key, rank)),
      isActive: props.filters[axis.key] === rank,
    })),
    {
      key: `${axis.key}-clear`,
      label: `Clear ${axis.label.toLowerCase()} filter`,
      run: () => props.setFilters({ ...props.filters, [axis.key]: null }),
      isActive: props.filters[axis.key] === null,
    },
  ]

  const commands = createMemo(() => [
    {
      key: 'only-violators',
      label: 'Show only violators',
      run: () =>
        props.setFilters({ ...props.filters, isViolatorsOnly: !props.filters.isViolatorsOnly }),
      isActive: props.filters.isViolatorsOnly,
    },
    {
      key: 'hide-di',
      label: 'Hide DI edges',
      run: () => toggleKind('di'),
      isActive: !props.filters.edgeKinds.has('di'),
    },
    {
      key: 'hide-runtime',
      label: 'Hide runtime edges',
      run: () => toggleKind('runtime'),
      isActive: !props.filters.edgeKinds.has('runtime'),
    },
    ...RANK_AXES.flatMap(rankCommands),
    ...COLOR_MODES.map((mode) => ({
      key: `color-${mode}`,
      label: `Colour by ${mode}`,
      run: () => props.setColorMode(mode),
      isActive: props.colorMode === mode,
    })),
  ])

  const matchedModules = createMemo(() => {
    const text = normalizePath(query().trim().toLowerCase())
    if (!text) return []
    return props.data.modules
      .filter(
        (m) =>
          m.id.toLowerCase().includes(text) ||
          normalizePath(m.path.toLowerCase()).includes(text),
      )
      .slice(0, MAX_PALETTE_MATCHES)
  })

  return (
    <>
      <button class="palette-btn" onClick={() => props.setPaletteOpen(true)}>
        <span class="palette-ico">⌕</span>
        <span class="palette-prompt">Search modules, run commands…</span>
        <span class="palette-kbd">⌘P</span>
      </button>
      <Show when={props.isPaletteOpen}>
        <div class="palette-scrim" onClick={() => props.setPaletteOpen(false)}>
          <div class="palette-dialog" onClick={(e) => e.stopPropagation()}>
            <div class="palette-input-row">
              <span class="palette-ico">⌕</span>
              <input
                ref={(el) => queueMicrotask(() => el.focus())}
                class="palette-input"
                placeholder="Type path, label, or > for commands…"
                value={query()}
                onInput={(e) => props.setFilters({ ...props.filters, query: e.currentTarget.value })}
              />
              <span class="palette-kbd palette-kbd-r">esc</span>
            </div>
            <div class="palette-results">
              <Show when={query().trim() === ''}>
                <For each={commands()}>
                  {(c) => (
                    <div
                      class={`palette-row palette-row-cmd ${c.isActive ? 'on' : ''}`}
                      onClick={() => c.run()}
                    >
                      <span class="palette-row-bullet">›</span>
                      <span class="palette-row-label">{c.label}</span>
                      <Show when={c.isActive}>
                        <span class="palette-row-tag">on</span>
                      </Show>
                    </div>
                  )}
                </For>
              </Show>
              <Show when={query().trim() !== '' && matchedModules().length === 0}>
                <div class="palette-empty">No matches.</div>
              </Show>
              <For each={matchedModules()}>
                {(m) => (
                  <div
                    class="palette-row"
                    onClick={() => {
                      props.onSelect(m.id)
                      props.setPaletteOpen(false)
                    }}
                  >
                    <span class="palette-row-bullet">▸</span>
                    <span class="palette-row-label">
                      <span class="palette-row-mono">
                        {m.container}/<b>{m.label}</b>
                      </span>
                    </span>
                    <Show when={m.severity === 'error'}>
                      <span class="palette-row-sev sev-err">err</span>
                    </Show>
                    <Show when={m.severity === 'warning'}>
                      <span class="palette-row-sev sev-warn">warn</span>
                    </Show>
                  </div>
                )}
              </For>
            </div>
            <div class="palette-foot">
              <span>
                <kbd>↑↓</kbd> navigate
              </span>
              <span>
                <kbd>↵</kbd> select
              </span>
              <span>
                <kbd>esc</kbd> close
              </span>
            </div>
          </div>
        </div>
      </Show>
    </>
  )
}

interface FacetBarProps {
  filters: Filters
  setFilters: (f: Filters) => void
  colorMode: ColorMode
  setColorMode: (m: ColorMode) => void
  data: DesignData
}

const FacetBar: Component<FacetBarProps> = (props) => {
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
      </div>
    </div>
  )
}
