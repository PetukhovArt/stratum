import {
  Component,
  createMemo,
  createSignal,
  For,
  JSX,
  onMount,
  Show,
} from 'solid-js'
import { createVirtualizer } from '@tanstack/solid-virtual'
import {
  DesignContainer,
  DesignData,
  DesignLayer,
  DesignModule,
  DesignViolation,
} from '../render/design'
import { Filters, HoveredEdge, HoveredMod } from '../state'
import { globToRegex, normalizePath } from './Graph'
import { Tooltip } from './Tooltip'

const isFilterActive = (f: Filters): boolean =>
  f.query !== '' || f.glob !== '' || f.onlyViolators || f.stageFilter !== null

// Set of module ids that should remain visible in the outline given the
// active filters. Returns null when no filter is active (everything shown).
// Matched modules pull their ancestors AND descendants into the visible set
// so the tree path to a match stays navigable, IDE-style.
const visibleOutlineModules = (data: DesignData, f: Filters): Set<string> | null => {
  if (!isFilterActive(f)) return null
  const matchGlob = globToRegex(f.glob)
  const q = f.query ? normalizePath(f.query.toLowerCase()) : null
  const byId = new Map<string, DesignModule>()
  for (const m of data.modules) byId.set(m.id, m)
  const direct = new Set<string>()
  for (const m of data.modules) {
    if (
      f.onlyViolators &&
      m.violations.length === 0 &&
      m.descError === 0 &&
      m.descWarn === 0
    )
      continue
    if (f.stageFilter !== null && m.stage !== f.stageFilter) continue
    if (q) {
      const pathLc = normalizePath(m.path.toLowerCase())
      if (!m.id.toLowerCase().includes(q) && !pathLc.includes(q)) continue
    }
    if (matchGlob && !matchGlob(m.path)) continue
    direct.add(m.id)
  }
  const visible = new Set<string>(direct)
  for (const id of direct) {
    let cur = byId.get(id)?.parentId ?? null
    while (cur) {
      if (visible.has(cur)) break
      visible.add(cur)
      cur = byId.get(cur)?.parentId ?? null
    }
  }
  const queue = [...direct]
  while (queue.length) {
    const id = queue.shift()!
    const kids = data.childIndex[id] ?? []
    for (const k of kids) {
      if (!visible.has(k)) {
        visible.add(k)
        queue.push(k)
      }
    }
  }
  return visible
}

const STAGE_NAMES: Record<number, string> = {
  1: 'file',
  2: 'folder',
  3: 'segments',
  4: 'composed',
}
const STAGE_DESC: Record<number, string> = {
  1: 'Stage 1 — single file. ≤2 exports, ~50–400 LOC. The smallest unit.',
  2: 'Stage 2 — flat folder + index. 2–6 files of one theme. Public API via index.ts.',
  3: 'Stage 3 — segments. Folder split into model / api / ui / lib.',
  4: 'Stage 4 — composed. composer + ≥2 sub-features + _shared. Internal structure becomes fractal.',
}
const stageName = (s: number) => STAGE_NAMES[s] ?? '?'
const stageDescription = (s: number) => STAGE_DESC[s] ?? ''

interface Tip {
  x: number
  y: number
  content: JSX.Element
}

// ─── IDE-style icons ──────────────────────────────────────────────────────

const FolderIcon: Component<{ open: boolean }> = (props) => (
  <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
    <Show
      when={props.open}
      fallback={
        <>
          <path
            d="M1.5 4 H5 L6 5 H12.5 V11.5 H1.5 Z"
            stroke="currentColor"
            stroke-width="1"
            stroke-linejoin="round"
          />
          <line x1="1.5" y1="6.5" x2="12.5" y2="6.5" stroke="currentColor" stroke-width="1" />
        </>
      }
    >
      <path
        d="M1.5 4.5 V11.5 H12.5 L13 6 H3 L2 4.5 Z"
        stroke="currentColor"
        stroke-width="1"
        stroke-linejoin="round"
      />
      <path
        d="M1.5 4.5 V11.5 H2.5 L3.5 6 H13"
        stroke="currentColor"
        stroke-width="1"
        fill="none"
        stroke-linejoin="round"
      />
    </Show>
  </svg>
)

const FileIcon: Component = () => (
  <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
    <path
      d="M3.5 1.5 H8 L11 4.5 V12.5 H3.5 Z"
      stroke="currentColor"
      stroke-width="1"
      stroke-linejoin="round"
    />
    <path
      d="M8 1.5 V4.5 H11"
      stroke="currentColor"
      stroke-width="1"
      stroke-linejoin="round"
    />
  </svg>
)

const ChevSVG: Component = () => (
  <svg width="8" height="8" viewBox="0 0 8 8">
    <path d="M2 1 L6 4 L2 7 Z" fill="currentColor" />
  </svg>
)

// ─── Outline ───────────────────────────────────────────────────────────────

interface OutlineProps {
  data: DesignData
  filters: Filters
  setFilters: (f: Filters) => void
  selectedId: string | null
  onSelect: (id: string) => void
}

export const OutlinePanel: Component<OutlineProps> = (props) => {
  // Start with deep children collapsed
  const [collapsed, setCollapsed] = createSignal<Set<string>>(
    new Set(props.data.modules.filter((m) => m.depth >= 1).map((m) => m.id)),
  )
  const [tip, setTip] = createSignal<Tip | null>(null)

  const visible = createMemo(() => visibleOutlineModules(props.data, props.filters))
  const filterActive = () => visible() !== null
  // When a glob/query/violator/stage filter is active, force-expand every
  // ancestor of a match so the user actually sees the matched leaves without
  // re-clicking chevrons.
  const isCollapsed = (id: string): boolean => {
    if (filterActive()) return false
    return collapsed().has(id)
  }
  const isModuleVisible = (id: string): boolean => {
    const v = visible()
    return v === null || v.has(id)
  }
  const visibleCount = createMemo(() => {
    const v = visible()
    if (!v) return props.data.modules.length
    let n = 0
    for (const m of props.data.modules) if (v.has(m.id)) n++
    return n
  })

  const showTip = (e: MouseEvent, content: JSX.Element) =>
    setTip({ x: e.clientX, y: e.clientY, content })
  const moveTip = (e: MouseEvent) => {
    const t = tip()
    if (t) setTip({ ...t, x: e.clientX, y: e.clientY })
  }
  const hideTip = () => setTip(null)

  const toggle = (id: string) => {
    const next = new Set(collapsed())
    if (next.has(id)) next.delete(id)
    else next.add(id)
    setCollapsed(next)
  }
  const toggleLayer = (layerId: string) => {
    const next = new Set(props.filters.disabledLayers)
    if (next.has(layerId)) next.delete(layerId)
    else next.add(layerId)
    props.setFilters({ ...props.filters, disabledLayers: next })
  }
  const toggleContainer = (cid: string) => {
    const next = new Set(props.filters.disabledContainers)
    if (next.has(cid)) next.delete(cid)
    else next.add(cid)
    props.setFilters({ ...props.filters, disabledContainers: next })
  }

  // Tooltip builders ──────────────────────────────────────────────────────
  const violationsTipContent = (
    scopeModules: DesignModule[],
    sev: 'error' | 'warning' | 'info',
    scopeLabel: string,
  ): JSX.Element => {
    const rules = new Map<string, number>()
    const sample: { rule: string; mod: string; msg: string }[] = []
    for (const m of scopeModules) {
      for (const vid of m.violations) {
        const v = props.data.violations.find((x) => x.id === vid)
        if (!v || v.severity !== sev) continue
        rules.set(v.rule, (rules.get(v.rule) ?? 0) + 1)
        if (sample.length < 3) sample.push({ rule: v.rule, mod: m.label, msg: v.message })
      }
    }
    if (rules.size === 0) return null
    const total = [...rules.values()].reduce((a, b) => a + b, 0)
    const sevClass = sev === 'error' ? 'sev-error' : sev === 'warning' ? 'sev-warning' : 'sev-info'
    return (
      <>
        <div class="tt-hd">
          <span class={`tt-sev ${sevClass}`}>{sev}</span>
          <span class="tt-name" style={{ margin: 0, 'font-size': '12px' }}>
            {scopeLabel}
          </span>
        </div>
        <div class="tt-vios" style={{ 'border-top': 'none', 'padding-top': 0 }}>
          {[...rules.entries()]
            .sort((a, b) => b[1] - a[1])
            .map(([r, n]) => (
              <div class="tt-vio-row">
                <span class={`tt-vio-tag ${sevClass}`}>×{n}</span>
                <span class="tt-vio-rule">{r}</span>
              </div>
            ))}
        </div>
        {sample.length > 0 && (
          <div
            style={{
              'margin-top': '8px',
              'padding-top': '8px',
              'border-top': '1px solid var(--bd-1)',
              display: 'flex',
              'flex-direction': 'column',
              gap: '4px',
            }}
          >
            {sample.map((s) => (
              <div style={{ 'font-size': '10.5px', color: 'var(--tx-3)' }}>
                <span style={{ 'font-family': "'JetBrains Mono', monospace", color: 'var(--tx-2)' }}>
                  {s.mod}
                </span>
                <span style={{ display: 'block', 'margin-top': '1px', 'line-height': 1.35 }}>
                  {s.msg}
                </span>
              </div>
            ))}
            {total > sample.length && (
              <div class="tt-vio-more">
                +{total - sample.length} more — click row to inspect
              </div>
            )}
          </div>
        )}
      </>
    )
  }

  const moduleViolationsTipContent = (mod: DesignModule): JSX.Element => {
    const vios = mod.violations
      .map((vid) => props.data.violations.find((x) => x.id === vid))
      .filter(Boolean) as DesignViolation[]
    if (vios.length === 0) return null
    return (
      <>
        <div class="tt-hd">
          <span class={`tt-sev sev-${vios[0].severity}`}>{vios[0].severity}</span>
          <span
            class="tt-name"
            style={{ margin: 0, 'font-size': '12px', 'font-family': "'JetBrains Mono', monospace" }}
          >
            {mod.label}
          </span>
        </div>
        <div style={{ display: 'flex', 'flex-direction': 'column', gap: '6px' }}>
          {vios.map((v) => (
            <div
              style={{
                'border-left': `2px solid var(--${
                  v.severity === 'error' ? 'err' : v.severity === 'warning' ? 'warn' : 'info'
                })`,
                'padding-left': '8px',
              }}
            >
              <div class="tt-vio-rule" style={{ 'margin-bottom': '2px' }}>
                {v.rule}
              </div>
              <div style={{ 'font-size': '11px', color: 'var(--tx-2)', 'line-height': 1.4 }}>
                {v.message}
              </div>
            </div>
          ))}
        </div>
      </>
    )
  }

  const layerCountTipContent = (layer: DesignLayer): JSX.Element => (
    <>
      <div class="tt-hd">
        <span
          class="tt-layer"
          style={{
            background: `oklch(70% 0.10 ${layer.hue} / .2)`,
            color: `oklch(82% 0.10 ${layer.hue})`,
          }}
        >
          {layer.label}
        </span>
      </div>
      <div class="tt-stats" style={{ 'margin-bottom': 0 }}>
        <span>{layer.modules} modules</span>
        {layer.errors > 0 && <span style={{ color: 'var(--err)' }}>{layer.errors} errors</span>}
        {layer.warnings > 0 && <span style={{ color: 'var(--warn)' }}>{layer.warnings} warnings</span>}
      </div>
    </>
  )

  const kidsTipContent = (n: number): JSX.Element => (
    <div style={{ 'font-size': '11.5px', color: 'var(--tx-2)', 'line-height': 1.45 }}>
      <b style={{ color: 'var(--tx-1)' }}>
        {n} nested module{n === 1 ? '' : 's'}
      </b>
      <div style={{ 'margin-top': '4px', color: 'var(--tx-3)' }}>
        This module is a compound — click chevron to expand.
      </div>
    </div>
  )

  const kindTipContent = (kind: 'composer' | 'shared'): JSX.Element => {
    const title = kind === 'composer' ? '◇ Composer' : '_ Shared'
    const body =
      kind === 'composer'
        ? 'Assembles its sibling sub-features into one composed unit. The composer is the public API of a stage-4 feature.'
        : 'Internal helpers reused by siblings of this composed feature. Not part of the public API.'
    return (
      <>
        <div class="tt-hd">
          <span class={`tt-kind tt-kind-${kind}`} style={{ 'font-size': '11px' }}>
            {title}
          </span>
        </div>
        <div style={{ 'font-size': '11.5px', color: 'var(--tx-2)', 'line-height': 1.45 }}>
          {body}
        </div>
      </>
    )
  }

  const clearFilters = () =>
    props.setFilters({ ...props.filters, query: '', glob: '', stageFilter: null, onlyViolators: false })

  // ─── Flatten the outline tree into a 1D array for virtualization ───────
  // Each FlatRow stores everything the renderer needs so the heavy compute
  // (filtering, ancestor expansion, alias detection) happens once per memo
  // invalidation, not per scroll frame.
  type FlatRow =
    | { type: 'layer'; layer: DesignLayer }
    | { type: 'container'; container: DesignContainer; layer: DesignLayer }
    | { type: 'module'; mod: DesignModule; rowDepth: number; kids: string[] }

  const modulesById = createMemo(() => {
    const m = new Map<string, DesignModule>()
    for (const mod of props.data.modules) m.set(mod.id, mod)
    return m
  })

  const containersByLayer = createMemo(() => {
    const m = new Map<string, DesignContainer[]>()
    for (const c of props.data.containers) {
      const list = m.get(c.layer)
      if (list) list.push(c)
      else m.set(c.layer, [c])
    }
    return m
  })

  const flatRows = createMemo<FlatRow[]>(() => {
    const out: FlatRow[] = []
    const byId = modulesById()
    const ctrByLayer = containersByLayer()

    const pushModuleSubtree = (modId: string, baseDepth: number, depth: number) => {
      if (!isModuleVisible(modId)) return
      const mod = byId.get(modId)
      if (!mod) return
      const kids = (props.data.childIndex[modId] ?? []).filter(isModuleVisible)
      out.push({ type: 'module', mod, rowDepth: baseDepth + depth, kids })
      if (kids.length === 0) return
      if (isCollapsed(modId)) return
      for (const kidId of kids) pushModuleSubtree(kidId, baseDepth, depth + 1)
    }

    for (const layer of props.data.layers) {
      const all = ctrByLayer.get(layer.id) ?? []
      const aliasContainer =
        all.length === 1 && all[0].label === layer.label ? all[0] : null
      const visibleContainers = all.filter((c) => {
        if (!filterActive()) return true
        return props.data.modules.some(
          (m) => m.container === c.id && isModuleVisible(m.id),
        )
      })
      if (filterActive() && visibleContainers.length === 0) continue
      out.push({ type: 'layer', layer })
      if (isCollapsed(layer.id)) continue
      if (aliasContainer) {
        // Skip the container row — Stratum's graph builder emits one root
        // container per layer with the layer's name; rendering it would be
        // pure noise.
        const tops = (props.data.topByContainer[aliasContainer.id] ?? []).filter(
          isModuleVisible,
        )
        for (const tid of tops) pushModuleSubtree(tid, 1, 0)
        continue
      }
      for (const c of visibleContainers) {
        out.push({ type: 'container', container: c, layer })
        if (isCollapsed(c.id)) continue
        const tops = (props.data.topByContainer[c.id] ?? []).filter(isModuleVisible)
        for (const tid of tops) pushModuleSubtree(tid, 2, 0)
      }
    }
    return out
  })

  // ─── Virtualizer ────────────────────────────────────────────────────────
  let scrollRef!: HTMLDivElement
  const virtualizer = createVirtualizer({
    get count() {
      return flatRows().length
    },
    getScrollElement: () => scrollRef,
    estimateSize: () => 22,
    overscan: 8,
  })

  // ─── Per-kind renderers ─────────────────────────────────────────────────
  const layerModulesCache = (layerId: string): DesignModule[] => {
    const ctrByLayer = containersByLayer()
    const ids = (ctrByLayer.get(layerId) ?? []).map((c) => c.id)
    return props.data.modules.filter((m) => ids.includes(m.container))
  }
  const containerModulesCache = (cId: string): DesignModule[] =>
    props.data.modules.filter((m) => m.container === cId)

  const renderLayerRow = (layer: DesignLayer) => {
    const layerCollapsed = () => isCollapsed(layer.id)
    const layerDisabled = () => props.filters.disabledLayers.has(layer.id)
    return (
      <div class="ot-row ot-row-layer" onClick={() => toggle(layer.id)}>
        <span
          class={`ot-chev ${layerCollapsed() ? 'is-collapsed' : 'is-open'}`}
          aria-hidden="true"
        >
          <ChevSVG />
        </span>
        <span class="ot-dot" style={{ background: `oklch(70% 0.10 ${layer.hue})` }} />
        <span class="ot-name">{layer.label}</span>
        <span class="ot-meta">
          <Show when={layer.errors > 0}>
            <span
              class="pill pill-err"
              onMouseEnter={(e) =>
                showTip(
                  e,
                  violationsTipContent(layerModulesCache(layer.id), 'error', `${layer.label} layer`),
                )
              }
              onMouseMove={moveTip}
              onMouseLeave={hideTip}
            >
              {layer.errors}
            </span>
          </Show>
          <Show when={layer.warnings > 0}>
            <span
              class="pill pill-warn"
              onMouseEnter={(e) =>
                showTip(
                  e,
                  violationsTipContent(
                    layerModulesCache(layer.id),
                    'warning',
                    `${layer.label} layer`,
                  ),
                )
              }
              onMouseMove={moveTip}
              onMouseLeave={hideTip}
            >
              {layer.warnings}
            </span>
          </Show>
          <span
            class="ot-num"
            onMouseEnter={(e) => showTip(e, layerCountTipContent(layer))}
            onMouseMove={moveTip}
            onMouseLeave={hideTip}
          >
            {layer.modules}
          </span>
        </span>
        <Tooltip content={layerDisabled() ? 'Show layer' : 'Hide layer'}>
          <button
            class={`ot-eye ${layerDisabled() ? 'off' : ''}`}
            onClick={(e) => {
              e.stopPropagation()
              toggleLayer(layer.id)
            }}
          >
            {layerDisabled() ? '○' : '●'}
          </button>
        </Tooltip>
      </div>
    )
  }

  const renderContainerRow = (c: DesignContainer) => {
    const cDisabled = () => props.filters.disabledContainers.has(c.id)
    const cCollapsed = () => isCollapsed(c.id)
    return (
      <div class="ot-row ot-row-ctr" onClick={() => toggle(c.id)}>
        <span
          class={`ot-chev ${cCollapsed() ? 'is-collapsed' : 'is-open'}`}
          aria-hidden="true"
        >
          <ChevSVG />
        </span>
        <span class="ot-icon" aria-hidden="true">
          <FolderIcon open={!cCollapsed()} />
        </span>
        <span class="ot-name ot-mono">{c.label}</span>
        <span class="ot-meta">
          <Show when={c.errors > 0}>
            <span
              class="pill pill-err"
              onMouseEnter={(e) =>
                showTip(e, violationsTipContent(containerModulesCache(c.id), 'error', c.label))
              }
              onMouseMove={moveTip}
              onMouseLeave={hideTip}
            >
              {c.errors}
            </span>
          </Show>
          <Show when={c.warnings > 0}>
            <span
              class="pill pill-warn"
              onMouseEnter={(e) =>
                showTip(e, violationsTipContent(containerModulesCache(c.id), 'warning', c.label))
              }
              onMouseMove={moveTip}
              onMouseLeave={hideTip}
            >
              {c.warnings}
            </span>
          </Show>
        </span>
        <button
          class={`ot-eye ${cDisabled() ? 'off' : ''}`}
          onClick={(e) => {
            e.stopPropagation()
            toggleContainer(c.id)
          }}
        >
          {cDisabled() ? '○' : '●'}
        </button>
      </div>
    )
  }

  const renderModuleRow = (m: DesignModule, rowDepth: number, kids: string[]) => {
    const hasKids = kids.length > 0
    const rowCollapsed = () => isCollapsed(m.id)
    const isSel = () => props.selectedId === m.id
    return (
      <div
        class={`ot-row ot-row-mod ${isSel() ? 'sel' : ''} kind-${m.kind}`}
        style={{ '--depth': rowDepth }}
        onClick={() => props.onSelect(m.id)}
      >
        <Show when={hasKids} fallback={<span class="ot-chev" aria-hidden="true" />}>
          <span
            class={`ot-chev ${rowCollapsed() ? 'is-collapsed' : 'is-open'}`}
            aria-hidden="true"
            onClick={(e) => {
              e.stopPropagation()
              toggle(m.id)
            }}
          >
            <ChevSVG />
          </span>
        </Show>
        <span class="ot-icon" aria-hidden="true">
          <Show when={hasKids} fallback={<FileIcon />}>
            <FolderIcon open={!rowCollapsed()} />
          </Show>
        </span>
        <Show when={m.kind === 'composer'}>
          <span
            class="ot-kind ot-kind-c"
            onMouseEnter={(e) => showTip(e, kindTipContent('composer'))}
            onMouseMove={moveTip}
            onMouseLeave={hideTip}
          >
            ◇
          </span>
        </Show>
        <Show when={m.kind === 'shared'}>
          <span
            class="ot-kind ot-kind-s"
            onMouseEnter={(e) => showTip(e, kindTipContent('shared'))}
            onMouseMove={moveTip}
            onMouseLeave={hideTip}
          >
            _
          </span>
        </Show>
        <span class="ot-mono ot-mod-name">{m.label}</span>
        <Show when={m.severity === 'error'}>
          <span
            class="ot-mod-sev sev-err"
            onMouseEnter={(e) => showTip(e, moduleViolationsTipContent(m))}
            onMouseMove={moveTip}
            onMouseLeave={hideTip}
          />
        </Show>
        <Show when={m.severity === 'warning'}>
          <span
            class="ot-mod-sev sev-warn"
            onMouseEnter={(e) => showTip(e, moduleViolationsTipContent(m))}
            onMouseMove={moveTip}
            onMouseLeave={hideTip}
          />
        </Show>
        <Show when={hasKids}>
          <span
            class="ot-mod-thick"
            onMouseEnter={(e) => showTip(e, kidsTipContent(kids.length))}
            onMouseMove={moveTip}
            onMouseLeave={hideTip}
          >
            {kids.length}
          </span>
        </Show>
      </div>
    )
  }

  return (
    <aside class="panel panel-outline">
      <header class="panel-hd">
        <span class="panel-title">Outline</span>
        <span class="panel-count">
          {filterActive() ? `${visibleCount()} / ${props.data.modules.length}` : props.data.modules.length}
        </span>
      </header>
      <div class="outline-tree" ref={scrollRef!} onScroll={hideTip}>
        <Show when={filterActive() && visibleCount() === 0}>
          <div class="ot-empty">
            <div class="ot-empty-icon">⌕</div>
            <div class="ot-empty-msg">No modules match the active filters.</div>
            <button class="ot-empty-clear" onClick={clearFilters}>
              Clear filters
            </button>
          </div>
        </Show>
        <Show when={!(filterActive() && visibleCount() === 0)}>
          <div style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }}>
            <For each={virtualizer.getVirtualItems()}>
              {(vi) => {
                const row = flatRows()[vi.index]
                if (!row) return null
                return (
                  <div
                    data-index={vi.index}
                    ref={(el) => queueMicrotask(() => virtualizer.measureElement(el))}
                    style={{
                      position: 'absolute',
                      top: 0,
                      left: 0,
                      right: 0,
                      transform: `translateY(${vi.start}px)`,
                    }}
                  >
                    {row.type === 'layer'
                      ? renderLayerRow(row.layer)
                      : row.type === 'container'
                        ? renderContainerRow(row.container)
                        : renderModuleRow(row.mod, row.rowDepth, row.kids)}
                  </div>
                )
              }}
            </For>
          </div>
        </Show>
      </div>
      <Show when={tip()}>{(t) => <TreeTooltip tip={t()} />}</Show>
    </aside>
  )
}

const TreeTooltip: Component<{ tip: Tip }> = (props) => {
  const [pos, setPos] = createSignal<{ left: number; top: number; ready: boolean }>({
    left: props.tip.x + 14,
    top: props.tip.y + 14,
    ready: false,
  })
  let ref!: HTMLDivElement
  onMount(() => {
    queueMicrotask(() => {
      if (!ref) return
      const w = ref.offsetWidth
      const h = ref.offsetHeight
      let left = props.tip.x + 14
      let top = props.tip.y + 14
      if (left + w > window.innerWidth - 8) left = props.tip.x - w - 14
      if (top + h > window.innerHeight - 8) top = props.tip.y - h - 14
      if (left < 8) left = 8
      if (top < 8) top = 8
      setPos({ left, top, ready: true })
    })
  })
  return (
    <div
      ref={ref}
      class="tt tree-tt"
      style={{
        position: 'fixed',
        left: pos().left + 'px',
        top: pos().top + 'px',
        'max-width': '320px',
        opacity: pos().ready ? 1 : 0,
      }}
    >
      {props.tip.content}
    </div>
  )
}


// ─── Details panel ────────────────────────────────────────────────────────

interface DetailsProps {
  data: DesignData
  selectedId: string | null
  /**
   * Kept on the type for API stability with App.tsx, but DELIBERATELY NOT
   * read inside the component. Switching the panel content on every hover
   * cratered perf — for a hub module the swap rebuilt hundreds of EdgeRow
   * nodes, each with an O(modules) lookup, costing ~500 ms per pointermove.
   * Hover preview lives in HoverTooltip; DetailsPanel responds to clicks.
   */
  hoveredId: string | null
  onClose: () => void
  onFocusCycle: (vid: string) => void
  focusedCycle: string | null
  onSelect: (id: string) => void
}

export const DetailsPanel: Component<DetailsProps> = (props) => {
  const id = () => props.selectedId
  const mod = () => (id() ? props.data.modules.find((m) => m.id === id()) ?? null : null)

  return (
    <Show
      when={mod()}
      fallback={
        <DetailsEmpty
          data={props.data}
          onFocusCycle={props.onFocusCycle}
          focusedCycle={props.focusedCycle}
        />
      }
    >
      {(m) => (
        <DetailsForModule
          mod={m()}
          data={props.data}
          selectedId={props.selectedId}
          onClose={props.onClose}
          onFocusCycle={props.onFocusCycle}
          focusedCycle={props.focusedCycle}
          onSelect={props.onSelect}
        />
      )}
    </Show>
  )
}

const DetailsForModule: Component<{
  mod: DesignModule
  data: DesignData
  selectedId: string | null
  onClose: () => void
  onFocusCycle: (vid: string) => void
  focusedCycle: string | null
  onSelect: (id: string) => void
}> = (props) => {
  const container = () => props.data.containers.find((c) => c.id === props.mod.container)
  const layer = () => props.data.layers.find((l) => l.id === container()?.layer)
  const incoming = createMemo(() => props.data.edges.filter((e) => e.target === props.mod.id))
  const outgoing = createMemo(() => props.data.edges.filter((e) => e.source === props.mod.id))
  const vios = createMemo(() =>
    props.mod.violations
      .map((vid) => props.data.violations.find((v) => v.id === vid))
      .filter((v): v is DesignViolation => !!v),
  )
  const kids = createMemo(() => props.data.childIndex[props.mod.id] ?? [])

  const ancestors = createMemo(() => {
    const out: DesignModule[] = []
    let cur = props.mod.parentId
    while (cur) {
      const p = props.data.modules.find((m) => m.id === cur)
      if (!p) break
      out.unshift(p)
      cur = p.parentId
    }
    return out
  })

  return (
    <aside class="panel panel-details">
      <header class="panel-hd">
        <span class="panel-title">
          <Show when={layer()}>
            {(l) => (
              <span
                class="hd-pill"
                style={{
                  background: `oklch(70% 0.10 ${l().hue} / .18)`,
                  color: `oklch(82% 0.10 ${l().hue})`,
                }}
              >
                {l().label}
              </span>
            )}
          </Show>
          <Show when={props.mod.kind === 'composer'}>
            <span class="hd-pill pill-kind-c">◇ composer</span>
          </Show>
          <Show when={props.mod.kind === 'shared'}>
            <span class="hd-pill pill-kind-s">_shared</span>
          </Show>
          <span class="panel-title-mono">{props.mod.label}</span>
        </span>
        <Show when={props.selectedId}>
          <button class="close-x" onClick={props.onClose}>
            ×
          </button>
        </Show>
      </header>
      <div class="panel-body">
        <div class="breadcrumb">
          <Show when={container()}>
            <span class="bc-item bc-ctr">{container()!.label}</span>
          </Show>
          <For each={ancestors()}>
            {(a) => (
              <>
                <span class="bc-sep">/</span>
                <span class="bc-item" onClick={() => props.onSelect(a.id)}>
                  {a.label}
                </span>
              </>
            )}
          </For>
          <span class="bc-sep">/</span>
          <span class="bc-item bc-self">{props.mod.label}</span>
        </div>
        <div class="det-path">{props.mod.path}</div>
        <div class="det-stats">
          <Stat label="LoC" value={props.mod.loc || '—'} title="Lines of code (when available)." />
          <Stat
            label="Stage"
            value={props.mod.stage}
            title={stageDescription(props.mod.stage)}
          />
          <Stat label="In" value={incoming().length} title="Incoming dependencies." />
          <Stat label="Out" value={outgoing().length} title="Outgoing dependencies." />
        </div>

        <Show when={kids().length > 0}>
          <Section title={`Children · ${kids().length}`}>
            <div class="kidgrid">
              <For each={kids()}>
                {(kid) => {
                  const k = props.data.modules.find((m) => m.id === kid)
                  if (!k) return null
                  const grandkids = props.data.childIndex[kid] ?? []
                  return (
                    <div
                      class={`kidgrid-item kind-${k.kind}`}
                      onClick={() => props.onSelect(kid)}
                    >
                      <div class="kidgrid-name">
                        <Show when={k.kind === 'composer'}>
                          <span class="kidgrid-tag">◇</span>
                        </Show>
                        <Show when={k.kind === 'shared'}>
                          <span class="kidgrid-tag">_</span>
                        </Show>
                        {k.label}
                      </div>
                      <div class="kidgrid-loc">
                        {k.loc ? `${k.loc} LoC · ` : ''}stage {k.stage}
                      </div>
                      <Show when={grandkids.length > 0}>
                        <div class="kidgrid-deep">+{grandkids.length} nested</div>
                      </Show>
                    </div>
                  )
                }}
              </For>
            </div>
          </Section>
        </Show>

        <Show when={vios().length > 0}>
          <Section title={`Violations · ${vios().length}`}>
            <div class="vios">
              <For each={vios()}>
                {(v) => (
                  <div class={`vio vio-${v.severity}`}>
                    <div class="vio-hd">
                      <span class={`vio-sev sev-${v.severity}`}>{v.severity}</span>
                      <span class="vio-rule">{v.rule}</span>
                      <Show when={v.rule.includes('cycle') || v.rule.includes('circular')}>
                        <button class="vio-focus" onClick={() => props.onFocusCycle(v.id)}>
                          {props.focusedCycle === v.id ? '◉ focused' : '⊙ focus'}
                        </button>
                      </Show>
                    </div>
                    <div class="vio-msg">{v.message}</div>
                    <div class="vio-doc">{v.ruleDoc}</div>
                  </div>
                )}
              </For>
            </div>
          </Section>
        </Show>

        <Section title={`Outgoing · ${outgoing().length}`}>
          <Show when={outgoing().length === 0}>
            <div class="empty">No outgoing imports.</div>
          </Show>
          <For each={outgoing()}>
            {(e) => (
              <EdgeRow
                kind={e.kind}
                violation={e.violation}
                otherId={e.target}
                dir="→"
                data={props.data}
                onSelect={props.onSelect}
              />
            )}
          </For>
        </Section>

        <Section title={`Incoming · ${incoming().length}`}>
          <Show when={incoming().length === 0}>
            <div class="empty">No dependents.</div>
          </Show>
          <For each={incoming()}>
            {(e) => (
              <EdgeRow
                kind={e.kind}
                violation={e.violation}
                otherId={e.source}
                dir="←"
                data={props.data}
                onSelect={props.onSelect}
              />
            )}
          </For>
        </Section>
      </div>
    </aside>
  )
}

const DetailsEmpty: Component<{
  data: DesignData
  onFocusCycle: (vid: string) => void
  focusedCycle: string | null
}> = (props) => {
  const totals = createMemo(() => {
    let err = 0
    let warn = 0
    let info = 0
    for (const v of props.data.violations) {
      if (v.severity === 'error') err++
      else if (v.severity === 'warning') warn++
      else info++
    }
    return { err, warn, info }
  })

  // Virtualize the All-violations list — on web-client this is ~366 rows ×
  // ~10 DOM nodes each, the single largest DOM source in the app.
  // Each row's height varies (message wraps), so we use a 84-px estimate
  // plus dynamic measurement via measureElement.
  let viosScrollRef!: HTMLDivElement
  const violations = createMemo(() => props.data.violations)
  const virtualizer = createVirtualizer({
    get count() {
      return violations().length
    },
    getScrollElement: () => viosScrollRef,
    estimateSize: () => 84,
    overscan: 6,
  })

  return (
    <aside class="panel panel-details">
      <header class="panel-hd">
        <span class="panel-title">
          <span class="panel-title-mono">{props.data.meta.project}</span>
        </span>
      </header>
      <div class="panel-body">
        <div class="snapshot-row">
          snapshot v{props.data.meta.snapshotVersion} · {props.data.meta.generatedAt.slice(0, 10)}
        </div>
        <div class="det-stats det-stats-grid">
          <Stat label="Modules" value={props.data.modules.length} />
          <Stat label="Containers" value={props.data.containers.length} />
          <Stat label="Edges" value={props.data.edges.length} />
          <Stat label="Errors" value={totals().err} sev="err" />
          <Stat label="Warnings" value={totals().warn} sev="warn" />
          <Stat label="Info" value={totals().info} sev="info" />
        </div>
        <Section title={`All violations · ${violations().length}`}>
          <div ref={viosScrollRef!} class="vios vios-virtual">
            <div style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }}>
              <For each={virtualizer.getVirtualItems()}>
                {(vi) => {
                  const v = violations()[vi.index]
                  if (!v) return null
                  return (
                    <div
                      data-index={vi.index}
                      ref={(el) => queueMicrotask(() => virtualizer.measureElement(el))}
                      class={`vio vio-${v.severity}`}
                      style={{
                        position: 'absolute',
                        top: 0,
                        left: 0,
                        right: 0,
                        transform: `translateY(${vi.start}px)`,
                      }}
                    >
                      <div class="vio-hd">
                        <span class={`vio-sev sev-${v.severity}`}>{v.severity}</span>
                        <span class="vio-rule">{v.rule}</span>
                        <Show when={v.rule.includes('cycle') || v.rule.includes('circular')}>
                          <button class="vio-focus" onClick={() => props.onFocusCycle(v.id)}>
                            {props.focusedCycle === v.id ? '◉ focused' : '⊙ focus'}
                          </button>
                        </Show>
                      </div>
                      <div class="vio-msg">{v.message}</div>
                    </div>
                  )
                }}
              </For>
            </div>
          </div>
        </Section>
        <Section title="Tip">
          <div class="empty">
            <div>Hover a module to inspect.</div>
            <div>Drag empty canvas to pan, scroll to zoom.</div>
            <div>
              <kbd>⌘P</kbd> · <kbd>/</kbd> command palette · <kbd>Esc</kbd> reset
            </div>
          </div>
        </Section>
      </div>
    </aside>
  )
}

const EdgeRow: Component<{
  kind: 'static' | 'di' | 'runtime'
  violation: string | null
  otherId: string
  dir: '→' | '←'
  data: DesignData
  onSelect: (id: string) => void
}> = (props) => {
  const other = () => props.data.modules.find((m) => m.id === props.otherId)
  const layer = () => {
    const o = other()
    if (!o) return null
    const c = props.data.containers.find((c) => c.id === o.container)
    return props.data.layers.find((l) => l.id === c?.layer) ?? null
  }
  const vio = () => (props.violation ? props.data.violations.find((v) => v.id === props.violation) : null)
  return (
    <Show when={other()}>
      {(o) => (
        <div
          class={`edge-row ${vio() ? 'edge-row-vio' : ''}`}
          onClick={() => props.onSelect(props.otherId)}
        >
          <span class="edge-row-dir">{props.dir}</span>
          <span class="edge-row-kind" data-kind={props.kind}>
            {props.kind}
          </span>
          <span class="edge-row-name">
            <Show when={layer()}>
              {(l) => (
                <span class="edge-row-layer" style={{ color: `oklch(78% 0.10 ${l().hue})` }}>
                  {l().label}
                </span>
              )}
            </Show>
            <span class="edge-row-mod"> / {o().label}</span>
          </span>
          <Show when={vio()}>
            {(v) => <span class={`edge-row-sev sev-${v().severity}`}>{v().severity}</span>}
          </Show>
        </div>
      )}
    </Show>
  )
}

const Section: Component<{ title: string; children: JSX.Element }> = (props) => (
  <section class="det-section">
    <h4>{props.title}</h4>
    {props.children}
  </section>
)

const Stat: Component<{
  label: string
  value: number | string
  sev?: 'err' | 'warn' | 'info'
  title?: string
}> = (props) => {
  const body = (
    <div class={`stat ${props.sev ? `stat-${props.sev}` : ''}`}>
      <div class="stat-val">{props.value}</div>
      <div class="stat-lbl">{props.label}</div>
    </div>
  )
  return (
    <Show when={props.title} fallback={body}>
      <Tooltip content={props.title!}>{body}</Tooltip>
    </Show>
  )
}

// ─── Hover tooltip (modules + edges) ───────────────────────────────────────

export const HoverTooltip: Component<{
  data: DesignData
  hoveredModule: HoveredMod | null
  hoveredEdge: HoveredEdge | null
  stageRect: DOMRect | null
}> = (props) => {
  return (
    <Show
      when={
        (props.hoveredModule || props.hoveredEdge) &&
        props.stageRect
      }
    >
      <TooltipBody
        data={props.data}
        hoveredModule={props.hoveredModule}
        hoveredEdge={props.hoveredEdge}
        stageRect={props.stageRect!}
      />
    </Show>
  )
}

const TooltipBody: Component<{
  data: DesignData
  hoveredModule: HoveredMod | null
  hoveredEdge: HoveredEdge | null
  stageRect: DOMRect
}> = (props) => {
  const isEdge = () => !!props.hoveredEdge && !props.hoveredModule
  const target = () => (isEdge() ? props.hoveredEdge! : props.hoveredModule!)
  const widthPx = () => (isEdge() ? 320 : 260)
  const baseX = () => target().x - props.stageRect.left + 14
  const baseY = () => target().y - props.stageRect.top + 14
  const finalX = () => Math.min(baseX(), props.stageRect.width - widthPx() - 8)
  const finalY = () => Math.min(baseY(), props.stageRect.height - 160)

  const mod = () => props.data.modules.find((m) => m.id === props.hoveredModule?.id)
  const modLayer = () => {
    const m = mod()
    if (!m) return null
    const c = props.data.containers.find((c) => c.id === m.container)
    return props.data.layers.find((l) => l.id === c?.layer) ?? null
  }
  const modVios = () =>
    (mod()?.violations ?? [])
      .map((vid) => props.data.violations.find((v) => v.id === vid))
      .filter((v): v is DesignViolation => !!v)
  const kidsCount = () => (mod() ? (props.data.childIndex[mod()!.id] ?? []).length : 0)
  const inCount = () => (mod() ? props.data.edges.filter((e) => e.target === mod()!.id).length : 0)
  const outCount = () => (mod() ? props.data.edges.filter((e) => e.source === mod()!.id).length : 0)

  return (
    <div
      class="tt"
      style={{
        left: finalX() + 'px',
        top: finalY() + 'px',
        width: widthPx() + 'px',
      }}
    >
      <Show when={isEdge()}>
        <EdgeTooltip edge={props.hoveredEdge!} data={props.data} />
      </Show>
      <Show when={!isEdge() && mod()}>
        {(m) => (
          <>
            <div class="tt-hd">
              <Show when={modLayer()}>
                {(l) => (
                  <span
                    class="tt-layer"
                    style={{
                      background: `oklch(70% 0.10 ${l().hue} / .2)`,
                      color: `oklch(82% 0.10 ${l().hue})`,
                    }}
                  >
                    {l().label}
                  </span>
                )}
              </Show>
              <Tooltip content={stageDescription(m().stage)}>
                <span class={`tt-stage tt-stage-${m().stage}`}>
                  ◔ {stageName(m().stage)}
                </span>
              </Tooltip>
              <Show when={m().kind === 'composer'}>
                <span class="tt-kind tt-kind-composer">◇ composer</span>
              </Show>
              <Show when={m().kind === 'shared'}>
                <span class="tt-kind tt-kind-shared">_shared</span>
              </Show>
              <Show when={m().severity}>
                <span class={`tt-sev sev-${m().severity}`}>{m().severity}</span>
              </Show>
            </div>
            <div class="tt-name">{m().label}</div>
            <div class="tt-path">{m().path}</div>
            <div class="tt-stats">
              <Show when={m().loc > 0}>
                <span>{m().loc} LoC</span>
              </Show>
              <Tooltip content={stageDescription(m().stage)}>
                <span>stage {m().stage} · {stageName(m().stage)}</span>
              </Tooltip>
              <span>in {inCount()}</span>
              <span>out {outCount()}</span>
              <Show when={kidsCount() > 0}>
                <span>{kidsCount()} children</span>
              </Show>
            </div>
            <Show when={modVios().length > 0}>
              <div class="tt-vios">
                <For each={modVios().slice(0, 3)}>
                  {(v) => (
                    <div class={`tt-vio-row sev-${v.severity}`}>
                      <span class={`tt-vio-tag sev-${v.severity}`}>{v.severity}</span>
                      <span class="tt-vio-rule">{v.rule}</span>
                    </div>
                  )}
                </For>
                <Show when={modVios().length > 3}>
                  <div class="tt-vio-more">
                    +{modVios().length - 3} more · click to inspect
                  </div>
                </Show>
              </div>
            </Show>
            <Show when={props.hoveredModule?.tiny}>
              <div class="tt-open-hint">click to open full card</div>
            </Show>
          </>
        )}
      </Show>
    </div>
  )
}

const EdgeTooltip: Component<{ edge: HoveredEdge; data: DesignData }> = (props) => {
  const src = () => props.data.modules.find((m) => m.id === props.edge.source)
  const dst = () => props.data.modules.find((m) => m.id === props.edge.target)
  const vio = () =>
    props.edge.violation
      ? props.data.violations.find((v) => v.id === props.edge.violation) ?? null
      : null
  return (
    <>
      <div class="tt-hd">
        <span class={`tt-kind tt-kind-${props.edge.kind}`}>{props.edge.kind}</span>
        <span class="tt-edge-arrow">→</span>
        <Show when={vio()}>
          {(v) => <span class={`tt-sev sev-${v().severity}`}>{v().severity}</span>}
        </Show>
      </div>
      <div class="tt-edge-route">
        <div class="tt-edge-end">{src()?.label ?? props.edge.source}</div>
        <div class="tt-edge-end">↓ {dst()?.label ?? props.edge.target}</div>
      </div>
      <Show when={vio()}>
        {(v) => (
          <div class="tt-vio">
            <div class="tt-vio-rule">{v().rule}</div>
            <div class="tt-vio-msg">{v().message}</div>
          </div>
        )}
      </Show>
    </>
  )
}
