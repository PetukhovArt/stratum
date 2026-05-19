import {
  Component,
  createMemo,
  createSignal,
  For,
  JSX,
  onMount,
  Show,
} from 'solid-js'
import {
  DesignData,
  DesignLayer,
  DesignModule,
  DesignViolation,
} from '../render/design'
import { Scene } from '../render/layout'
import { Filters, HoveredEdge, HoveredMod, StageSize, Viewport } from '../state'
import { globToRegex, normalizePath } from './Graph'

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

  const containersOf = (layer: DesignLayer) =>
    props.data.containers.filter((c) => c.layer === layer.id)

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

  return (
    <aside class="panel panel-outline">
      <header class="panel-hd">
        <span class="panel-title">Outline</span>
        <span class="panel-count">
          {filterActive() ? `${visibleCount()} / ${props.data.modules.length}` : props.data.modules.length}
        </span>
      </header>
      <div class="outline-tree" onScroll={hideTip}>
        <For each={props.data.layers}>
          {(layer) => {
            const layerCollapsed = () => isCollapsed(layer.id)
            const layerDisabled = () => props.filters.disabledLayers.has(layer.id)
            const layerModules = createMemo(() =>
              props.data.modules.filter((m) => {
                const c = props.data.containers.find((c) => c.id === m.container)
                return c?.layer === layer.id
              }),
            )
            const visibleContainers = createMemo(() =>
              containersOf(layer).filter((c) => {
                if (!filterActive()) return true
                return props.data.modules.some(
                  (m) => m.container === c.id && isModuleVisible(m.id),
                )
              }),
            )
            if (filterActive() && visibleContainers().length === 0) return null
            return (
              <div class="ot-layer">
                <div class="ot-row ot-row-layer" onClick={() => toggle(layer.id)}>
                  <span
                    class={`ot-chev ${layerCollapsed() ? 'is-collapsed' : 'is-open'}`}
                    aria-hidden="true"
                  >
                    <svg width="8" height="8" viewBox="0 0 8 8">
                      <path d="M2 1 L6 4 L2 7 Z" fill="currentColor" />
                    </svg>
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
                            violationsTipContent(layerModules(), 'error', `${layer.label} layer`),
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
                              layerModules(),
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
                  <button
                    class={`ot-eye ${layerDisabled() ? 'off' : ''}`}
                    onClick={(e) => {
                      e.stopPropagation()
                      toggleLayer(layer.id)
                    }}
                    title={layerDisabled() ? 'show layer' : 'hide layer'}
                  >
                    {layerDisabled() ? '○' : '●'}
                  </button>
                </div>
                <Show when={!layerCollapsed()}>
                  <For each={visibleContainers()}>
                    {(c) => {
                      const cDisabled = () => props.filters.disabledContainers.has(c.id)
                      const cCollapsed = () => isCollapsed(c.id)
                      const tops = () =>
                        (props.data.topByContainer[c.id] ?? []).filter(isModuleVisible)
                      const containerModules = createMemo(() =>
                        props.data.modules.filter((m) => m.container === c.id),
                      )
                      return (
                        <div class="ot-container">
                          <div class="ot-row ot-row-ctr" onClick={() => toggle(c.id)}>
                            <span
                              class={`ot-chev ${cCollapsed() ? 'is-collapsed' : 'is-open'}`}
                              aria-hidden="true"
                            >
                              <svg width="8" height="8" viewBox="0 0 8 8">
                                <path d="M2 1 L6 4 L2 7 Z" fill="currentColor" />
                              </svg>
                            </span>
                            <span class="ot-name ot-mono">{c.label}</span>
                            <span class="ot-meta">
                              <Show when={c.errors > 0}>
                                <span
                                  class="pill pill-err"
                                  onMouseEnter={(e) =>
                                    showTip(
                                      e,
                                      violationsTipContent(containerModules(), 'error', c.label),
                                    )
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
                                    showTip(
                                      e,
                                      violationsTipContent(
                                        containerModules(),
                                        'warning',
                                        c.label,
                                      ),
                                    )
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
                          <Show when={!cCollapsed()}>
                            <For each={tops()}>
                              {(modId) => (
                                <ModuleRow
                                  modId={modId}
                                  data={props.data}
                                  collapsed={collapsed}
                                  toggle={toggle}
                                  selectedId={props.selectedId}
                                  onSelect={props.onSelect}
                                  depth={0}
                                  showTip={showTip}
                                  moveTip={moveTip}
                                  hideTip={hideTip}
                                  moduleViolationsTipContent={moduleViolationsTipContent}
                                  kidsTipContent={kidsTipContent}
                                  isModuleVisible={isModuleVisible}
                                  isCollapsed={isCollapsed}
                                />
                              )}
                            </For>
                          </Show>
                        </div>
                      )
                    }}
                  </For>
                </Show>
              </div>
            )
          }}
        </For>
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

interface ModuleRowProps {
  modId: string
  data: DesignData
  collapsed: () => Set<string>
  toggle: (id: string) => void
  selectedId: string | null
  onSelect: (id: string) => void
  depth: number
  showTip: (e: MouseEvent, content: JSX.Element) => void
  moveTip: (e: MouseEvent) => void
  hideTip: () => void
  moduleViolationsTipContent: (mod: DesignModule) => JSX.Element
  kidsTipContent: (n: number) => JSX.Element
  isModuleVisible: (id: string) => boolean
  isCollapsed: (id: string) => boolean
}

const ModuleRow: Component<ModuleRowProps> = (props) => {
  const mod = () => props.data.modules.find((m) => m.id === props.modId)
  const kids = () => (props.data.childIndex[props.modId] ?? []).filter(props.isModuleVisible)
  const hasKids = () => kids().length > 0
  const isCollapsed = () => props.isCollapsed(props.modId)

  return (
    <Show when={mod()}>
      {(m) => (
        <>
          <div
            class={`ot-row ot-row-mod ${
              props.selectedId === props.modId ? 'sel' : ''
            } kind-${m().kind}`}
            style={{ 'padding-left': `${28 + props.depth * 14}px` }}
            onClick={() => props.onSelect(props.modId)}
          >
            <Show
              when={hasKids()}
              fallback={<span class="ot-chev" aria-hidden="true" />}
            >
              <span
                class={`ot-chev ${isCollapsed() ? 'is-collapsed' : 'is-open'}`}
                aria-hidden="true"
                onClick={(e) => {
                  e.stopPropagation()
                  props.toggle(props.modId)
                }}
              >
                <svg width="8" height="8" viewBox="0 0 8 8">
                  <path d="M2 1 L6 4 L2 7 Z" fill="currentColor" />
                </svg>
              </span>
            </Show>
            <Show when={m().kind === 'composer'}>
              <span class="ot-kind ot-kind-c">◇</span>
            </Show>
            <Show when={m().kind === 'shared'}>
              <span class="ot-kind ot-kind-s">_</span>
            </Show>
            <span class="ot-mono ot-mod-name">{m().label}</span>
            <Show when={m().severity === 'error'}>
              <span
                class="ot-mod-sev sev-err"
                onMouseEnter={(e) => props.showTip(e, props.moduleViolationsTipContent(m()))}
                onMouseMove={props.moveTip}
                onMouseLeave={props.hideTip}
              />
            </Show>
            <Show when={m().severity === 'warning'}>
              <span
                class="ot-mod-sev sev-warn"
                onMouseEnter={(e) => props.showTip(e, props.moduleViolationsTipContent(m()))}
                onMouseMove={props.moveTip}
                onMouseLeave={props.hideTip}
              />
            </Show>
            <Show when={hasKids()}>
              <span
                class="ot-mod-thick"
                onMouseEnter={(e) => props.showTip(e, props.kidsTipContent(kids().length))}
                onMouseMove={props.moveTip}
                onMouseLeave={props.hideTip}
              >
                {kids().length}
              </span>
            </Show>
          </div>
          <Show when={hasKids() && !isCollapsed()}>
            <For each={kids()}>
              {(kidId) => (
                <ModuleRow
                  modId={kidId}
                  data={props.data}
                  collapsed={props.collapsed}
                  toggle={props.toggle}
                  selectedId={props.selectedId}
                  onSelect={props.onSelect}
                  depth={props.depth + 1}
                  showTip={props.showTip}
                  moveTip={props.moveTip}
                  hideTip={props.hideTip}
                  moduleViolationsTipContent={props.moduleViolationsTipContent}
                  kidsTipContent={props.kidsTipContent}
                  isModuleVisible={props.isModuleVisible}
                  isCollapsed={props.isCollapsed}
                />
              )}
            </For>
          </Show>
        </>
      )}
    </Show>
  )
}

// ─── Details panel ────────────────────────────────────────────────────────

interface DetailsProps {
  data: DesignData
  selectedId: string | null
  hoveredId: string | null
  onClose: () => void
  onFocusCycle: (vid: string) => void
  focusedCycle: string | null
  onSelect: (id: string) => void
}

export const DetailsPanel: Component<DetailsProps> = (props) => {
  const id = () => props.selectedId ?? props.hoveredId
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
        <Section title={`All violations · ${props.data.violations.length}`}>
          <div class="vios">
            <For each={props.data.violations}>
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
                </div>
              )}
            </For>
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
}> = (props) => (
  <div class={`stat ${props.sev ? `stat-${props.sev}` : ''}`} title={props.title}>
    <div class="stat-val">{props.value}</div>
    <div class="stat-lbl">{props.label}</div>
  </div>
)

// ─── Minimap ───────────────────────────────────────────────────────────────

export const Minimap: Component<{
  scene: Scene
  viewport: Viewport
  onViewportChange: (v: Viewport) => void
  containerSize: StageSize
}> = (props) => {
  const W = 200
  const H = 140
  const scale = () => {
    const sx = W / Math.max(props.scene.width, 1)
    const sy = H / Math.max(props.scene.height, 1)
    return Math.min(sx, sy)
  }
  const offsetX = () => (W - props.scene.width * scale()) / 2
  const offsetY = () => (H - props.scene.height * scale()) / 2
  const vw = () => props.containerSize.w / props.viewport.zoom
  const vh = () => props.containerSize.h / props.viewport.zoom
  const vx = () => -props.viewport.x / props.viewport.zoom
  const vy = () => -props.viewport.y / props.viewport.zoom

  const onClick = (e: MouseEvent) => {
    const r = (e.currentTarget as Element).getBoundingClientRect()
    const mx = e.clientX - r.left
    const my = e.clientY - r.top
    const sx = (mx - offsetX()) / scale()
    const sy = (my - offsetY()) / scale()
    props.onViewportChange({
      ...props.viewport,
      x: -sx * props.viewport.zoom + props.containerSize.w / 2,
      y: -sy * props.viewport.zoom + props.containerSize.h / 2,
    })
  }

  return (
    <div class="minimap">
      <svg width={W} height={H} onClick={onClick}>
        <rect x={0} y={0} width={W} height={H} fill="rgba(0,0,0,0.4)" rx={6} />
        <g transform={`translate(${offsetX()}, ${offsetY()}) scale(${scale()})`}>
          <For each={props.scene.lanes}>
            {(lane) => (
              <rect
                x={0}
                y={lane.y}
                width={props.scene.width}
                height={lane.height}
                fill={`oklch(60% 0.08 ${lane.hue} / .18)`}
              />
            )}
          </For>
          <For each={props.scene.containers}>
            {(c) => (
              <rect
                x={c.absX}
                y={c.absY}
                width={c.width}
                height={c.height}
                fill="rgba(255,255,255,0.05)"
                stroke="rgba(255,255,255,0.10)"
                stroke-width={1 / scale()}
              />
            )}
          </For>
          <For each={Object.entries(props.scene.modulePos)}>
            {([, p]) => (
              <Show when={p.mod.severity}>
                <rect
                  x={p.x}
                  y={p.y}
                  width={p.w}
                  height={p.h}
                  fill={
                    p.mod.severity === 'error'
                      ? 'oklch(64% 0.18 25)'
                      : 'oklch(78% 0.14 70)'
                  }
                />
              </Show>
            )}
          </For>
        </g>
        <rect
          x={offsetX() + vx() * scale()}
          y={offsetY() + vy() * scale()}
          width={vw() * scale()}
          height={vh() * scale()}
          fill="rgba(255,255,255,0.06)"
          stroke="oklch(85% 0.10 65)"
          stroke-width={1.2}
          rx={2}
          pointer-events="none"
        />
      </svg>
      <div class="minimap-hint">click to recenter</div>
    </div>
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
              <span class={`tt-stage tt-stage-${m().stage}`} title={stageDescription(m().stage)}>
                ◔ {stageName(m().stage)}
              </span>
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
              <span title={stageDescription(m().stage)}>
                stage {m().stage} · {stageName(m().stage)}
              </span>
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
