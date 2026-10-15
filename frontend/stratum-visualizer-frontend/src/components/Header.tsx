import { Component, createMemo, For, Show } from 'solid-js'
import { activeFilterCount, clearFilters, toggled } from '../lib/filters'
import { normalizePath } from '../lib/glob'
import { DesignData, EdgeKindLabel } from '../render/design'
import { ALL_EDGE_KINDS, Filters } from '../state'

const MAX_PALETTE_MATCHES = 20

const EDGE_KIND_TOOLTIP: Record<EdgeKindLabel, string> = {
  static: 'Static imports — resolved at build time (import / export).',
  di: 'DI edges — injected at runtime through your container/provider.',
  runtime: 'Runtime edges — lazy / dynamic imports, conditional loads.',
}

interface HeaderProps {
  data: DesignData
  filters: Filters
  setFilters: (f: Filters) => void
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

      <FacetBar filters={props.filters} setFilters={props.setFilters} data={props.data} />
    </header>
  )
}

interface PaletteProps {
  isPaletteOpen: boolean
  setPaletteOpen: (v: boolean) => void
  filters: Filters
  setFilters: (f: Filters) => void
  data: DesignData
  onSelect: (id: string) => void
}

const CommandPalette: Component<PaletteProps> = (props) => {
  const query = () => props.filters.query
  const toggleKind = (kind: EdgeKindLabel) =>
    props.setFilters({ ...props.filters, edgeKinds: toggled(props.filters.edgeKinds, kind) })
  const setStage = (s: number | null) => {
    props.setFilters({ ...props.filters, stageFilter: s })
  }

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
    {
      key: 'stage-1',
      label: 'Filter by stage: 1 (file)',
      run: () => setStage(props.filters.stageFilter === 1 ? null : 1),
      isActive: props.filters.stageFilter === 1,
    },
    {
      key: 'stage-2',
      label: 'Filter by stage: 2 (folder)',
      run: () => setStage(props.filters.stageFilter === 2 ? null : 2),
      isActive: props.filters.stageFilter === 2,
    },
    {
      key: 'stage-3',
      label: 'Filter by stage: 3 (segments)',
      run: () => setStage(props.filters.stageFilter === 3 ? null : 3),
      isActive: props.filters.stageFilter === 3,
    },
    {
      key: 'clear-stage',
      label: 'Clear stage filter',
      run: () => setStage(null),
      isActive: props.filters.stageFilter === null,
    },
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
        <Show when={props.filters.stageFilter !== null}>
          <span class="facet-sep" />
          <div class="facet-group">
            <button
              class="chip on"
              onClick={() => props.setFilters({ ...props.filters, stageFilter: null })}
            >
              <span class="chip-name">stage = {props.filters.stageFilter} ✕</span>
            </button>
          </div>
        </Show>
        <span class="facet-sep" />
        <div class="facet-group facet-glob">
          <span class="facet-label">Path</span>
          <input
            class="glob-input"
            placeholder="gis , src/features/** , !**/legacy/**"
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
    </div>
  )
}
