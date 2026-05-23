import { Component, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { DesignData } from '../render/design'
import { Filters } from '../state'
import { normalizePath } from './Graph'

const EDGE_KIND_TOOLTIP: Record<string, string> = {
  static: 'Static imports — resolved at build time (import / export).',
  di: 'DI edges — injected at runtime through your container/provider.',
  runtime: 'Runtime edges — lazy / dynamic imports, conditional loads.',
}

interface HeaderProps {
  data: DesignData
  filters: Filters
  setFilters: (f: Filters) => void
  paletteOpen: boolean
  setPaletteOpen: (v: boolean) => void
  onSelect: (id: string) => void
  onResetView: () => void
  onTweaksToggle: () => void
  renderer: 'svg' | 'webgl'
  onRendererToggle: () => void
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
        <HeaderPathInput filters={props.filters} setFilters={props.setFilters} />
      </div>
      <CommandPalette
        paletteOpen={props.paletteOpen}
        setPaletteOpen={props.setPaletteOpen}
        filters={props.filters}
        setFilters={props.setFilters}
        data={props.data}
        onSelect={props.onSelect}
      />

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
          <button
            class="hd-btn hd-btn-renderer"
            onClick={props.onRendererToggle}
            title={`Renderer: ${props.renderer.toUpperCase()} (click to switch)`}
          >
            {props.renderer === 'webgl' ? '🌐 WebGL' : '◇ SVG'}
          </button>
          <button class="hd-btn" onClick={props.onResetView} title="Fit to bounds (Esc)">
            ⤢ fit
          </button>
          <button class="hd-btn" onClick={props.onTweaksToggle} title="Open tweaks">
            ⚙
          </button>
        </div>
      </div>

      <FacetBar filters={props.filters} setFilters={props.setFilters} data={props.data} />
    </header>
  )
}

// ─── Header path input ─────────────────────────────────────────────────────
// Primary path filter, sits in the top bar. Supports the same syntax as the
// old FacetBar `Path` input: substring (`gis`), glob (`src/**`), negation
// (`!**/legacy/**`), comma-separated. Synced with `filters.glob` so the
// graph + outline pick up the filter directly.

const HeaderPathInput: Component<{
  filters: Filters
  setFilters: (f: Filters) => void
}> = (props) => (
  <div class="hd-path">
    <span class="hd-path-ico">⌕</span>
    <input
      class="hd-path-input"
      placeholder="Filter by path — gis , src/features/** , !**/legacy/**"
      value={props.filters.glob}
      onInput={(e) => props.setFilters({ ...props.filters, glob: e.currentTarget.value })}
      title="Bare words match as case-insensitive substring. Globs (`*`, `**`, `?`) match path segments. Comma-separated. Prefix `!` to exclude."
    />
    <Show when={props.filters.glob !== ''}>
      <button
        class="hd-path-clear"
        onClick={() => props.setFilters({ ...props.filters, glob: '' })}
        title="Clear path filter"
      >
        ✕
      </button>
    </Show>
    <span class="hd-path-kbd" title="Open command palette">⌘P</span>
  </div>
)

// ─── Command palette ───────────────────────────────────────────────────────

interface PaletteProps {
  paletteOpen: boolean
  setPaletteOpen: (v: boolean) => void
  filters: Filters
  setFilters: (f: Filters) => void
  data: DesignData
  onSelect: (id: string) => void
}

const CommandPalette: Component<PaletteProps> = (props) => {
  const [q, setQ] = createSignal('')
  let inputRef!: HTMLInputElement

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && (e.key === 'p' || e.key === 'k')) {
        e.preventDefault()
        props.setPaletteOpen(true)
      } else if (e.key === 'Escape') {
        props.setPaletteOpen(false)
      } else if (
        e.key === '/' &&
        document.activeElement?.tagName !== 'INPUT' &&
        document.activeElement?.tagName !== 'TEXTAREA'
      ) {
        e.preventDefault()
        props.setPaletteOpen(true)
      }
    }
    window.addEventListener('keydown', onKey)
    onCleanup(() => window.removeEventListener('keydown', onKey))
  })

  const focusInputSoon = () => setTimeout(() => inputRef?.focus(), 30)

  const toggleKind = (kind: 'static' | 'di' | 'runtime') => {
    const next = new Set(props.filters.edgeKinds)
    if (next.has(kind)) next.delete(kind)
    else next.add(kind)
    props.setFilters({ ...props.filters, edgeKinds: next })
  }
  const setStage = (s: number | null) => {
    props.setFilters({ ...props.filters, stageFilter: s })
  }

  const commands = createMemo(() => [
    {
      key: 'only-violators',
      label: 'Show only violators',
      run: () =>
        props.setFilters({ ...props.filters, onlyViolators: !props.filters.onlyViolators }),
      active: props.filters.onlyViolators,
    },
    {
      key: 'hide-di',
      label: 'Hide DI edges',
      run: () => toggleKind('di'),
      active: !props.filters.edgeKinds.has('di'),
    },
    {
      key: 'hide-runtime',
      label: 'Hide runtime edges',
      run: () => toggleKind('runtime'),
      active: !props.filters.edgeKinds.has('runtime'),
    },
    {
      key: 'stage-1',
      label: 'Filter by stage: 1 (file)',
      run: () => setStage(props.filters.stageFilter === 1 ? null : 1),
      active: props.filters.stageFilter === 1,
    },
    {
      key: 'stage-2',
      label: 'Filter by stage: 2 (folder)',
      run: () => setStage(props.filters.stageFilter === 2 ? null : 2),
      active: props.filters.stageFilter === 2,
    },
    {
      key: 'stage-3',
      label: 'Filter by stage: 3 (segments)',
      run: () => setStage(props.filters.stageFilter === 3 ? null : 3),
      active: props.filters.stageFilter === 3,
    },
    {
      key: 'clear-stage',
      label: 'Clear stage filter',
      run: () => setStage(null),
      active: props.filters.stageFilter === null,
    },
  ])

  const matchedModules = createMemo(() => {
    const text = normalizePath(q().trim().toLowerCase())
    if (!text) return []
    return props.data.modules
      .filter(
        (m) =>
          m.id.toLowerCase().includes(text) ||
          normalizePath(m.path.toLowerCase()).includes(text),
      )
      .slice(0, 20)
  })

  // Keep the trigger out of the DOM — the palette is now keyboard-only
  // (⌘P / ⌘K / `/`). focusInputSoon stays to focus the modal input on open.
  void focusInputSoon
  return (
    <>
      <Show when={props.paletteOpen}>
        <div class="palette-scrim" onClick={() => props.setPaletteOpen(false)}>
          <div class="palette-dialog" onClick={(e) => e.stopPropagation()}>
            <div class="palette-input-row">
              <span class="palette-ico">⌕</span>
              <input
                ref={(el) => {
                  inputRef = el
                  focusInputSoon()
                }}
                class="palette-input"
                placeholder="Type path, label, or > for commands…"
                value={q()}
                onInput={(e) => {
                  setQ(e.currentTarget.value)
                  props.setFilters({ ...props.filters, query: e.currentTarget.value })
                }}
              />
              <span class="palette-kbd palette-kbd-r">esc</span>
            </div>
            <div class="palette-results">
              <Show when={q().trim() === ''}>
                <For each={commands()}>
                  {(c) => (
                    <div
                      class={`palette-row palette-row-cmd ${c.active ? 'on' : ''}`}
                      onClick={() => c.run()}
                    >
                      <span class="palette-row-bullet">›</span>
                      <span class="palette-row-label">{c.label}</span>
                      <Show when={c.active}>
                        <span class="palette-row-tag">on</span>
                      </Show>
                    </div>
                  )}
                </For>
              </Show>
              <Show when={q().trim() !== '' && matchedModules().length === 0}>
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

// ─── Facet chip bar ─────────────────────────────────────────────────────────

interface FacetBarProps {
  filters: Filters
  setFilters: (f: Filters) => void
  data: DesignData
}

const FacetBar: Component<FacetBarProps> = (props) => {
  const toggleLayer = (id: string) => {
    const next = new Set(props.filters.disabledLayers)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    props.setFilters({ ...props.filters, disabledLayers: next })
  }
  const toggleKind = (k: 'static' | 'di' | 'runtime') => {
    const next = new Set(props.filters.edgeKinds)
    if (next.has(k)) next.delete(k)
    else next.add(k)
    props.setFilters({ ...props.filters, edgeKinds: next })
  }

  const hasActiveFilters = createMemo(
    () =>
      props.filters.disabledLayers.size > 0 ||
      props.filters.disabledContainers.size > 0 ||
      props.filters.onlyViolators ||
      props.filters.query !== '' ||
      props.filters.glob !== '' ||
      props.filters.stageFilter !== null,
  )

  const clearAll = () =>
    props.setFilters({
      ...props.filters,
      disabledLayers: new Set(),
      disabledContainers: new Set(),
      onlyViolators: false,
      query: '',
      glob: '',
      stageFilter: null,
    })

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
          <For each={['static', 'di', 'runtime'] as const}>
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
            class={`chip chip-mode ${props.filters.onlyViolators ? 'on' : ''}`}
            onClick={() =>
              props.setFilters({ ...props.filters, onlyViolators: !props.filters.onlyViolators })
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
        <Show when={hasActiveFilters()}>
          <button class="facet-clear" onClick={clearAll}>
            clear all
          </button>
        </Show>
      </div>
    </div>
  )
}
