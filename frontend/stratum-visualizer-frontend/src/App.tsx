import {
  Component,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  onCleanup,
  onMount,
  Show,
} from 'solid-js'
import './styles.css'

import { Graph } from './components/Graph'
import { GraphWebGL } from './components/GraphWebGL'
import { Header } from './components/Header'
import { ModuleCard } from './components/ModuleCard'
import { DetailsPanel, HoverTooltip, OutlinePanel } from './components/Panels'
import { Legend, StatusBar, ZoomControls } from './components/StatusBar'
import { loadSnapshot, SnapshotVersionMismatchError } from './lib/snapshot'
import { loadViolations } from './lib/violations'
import { adaptSnapshot } from './render/adapt'
import { DesignData } from './render/design'
import { computeLayout } from './render/layout'
import {
  DEFAULT_FILTERS,
  DEFAULT_TWEAKS,
  Filters,
  HoveredEdge,
  HoveredMod,
  Viewport,
} from './state'
import { GraphSnapshot, Violation } from './types'

const readInitialRenderer = (): 'svg' | 'webgl' => {
  const fromUrl = new URLSearchParams(window.location.search).get('renderer')
  if (fromUrl === 'webgl' || fromUrl === 'svg') return fromUrl
  try {
    const v = localStorage.getItem('stratum.renderer')
    if (v === 'webgl' || v === 'svg') return v
  } catch (_e) {
    /* ignore */
  }
  return 'svg'
}

const App: Component = () => {
  const [snapshot] = createResource<GraphSnapshot>(() => loadSnapshot('/api/snapshot'))
  const [violations] = createResource<Violation[]>(() =>
    loadViolations('/api/violations').catch(() => [] as Violation[]),
  )

  const data = createMemo<DesignData | null>(() => {
    const s = snapshot()
    if (!s) return null
    return adaptSnapshot(s, violations() ?? [], { project: deriveProjectName(s) })
  })

  const [filters, setFilters] = createSignal<Filters>(DEFAULT_FILTERS())
  const [tweaks, setTweaks] = createSignal(DEFAULT_TWEAKS)
  const [selected, setSelected] = createSignal<string | null>(null)
  const [hovered, setHovered] = createSignal<HoveredMod | null>(null)
  const [hoveredEdge, setHoveredEdge] = createSignal<HoveredEdge | null>(null)
  const [paletteOpen, setPaletteOpen] = createSignal(false)
  const [focusedCycle, setFocusedCycle] = createSignal<string | null>(null)
  const [cardModuleId, setCardModuleId] = createSignal<string | null>(null)
  const [leftCollapsed, setLeftCollapsed] = createSignal(false)
  const [rightCollapsed, setRightCollapsed] = createSignal(false)
  const [leftWidth, setLeftWidth] = createSignal(264)
  const [rightWidth, setRightWidth] = createSignal(340)
  const [viewport, setViewport] = createSignal<Viewport>({ x: 0, y: 0, zoom: 0.5 })
  const [stageSize, setStageSize] = createSignal({ w: 1000, h: 700 })
  const [stageRect, setStageRect] = createSignal<DOMRect | null>(null)
  const [hasFitted, setHasFitted] = createSignal(false)
  const [renderer, setRenderer] = createSignal<'svg' | 'webgl'>(readInitialRenderer())
  createEffect(() => {
    const r = renderer()
    const url = new URL(window.location.href)
    url.searchParams.set('renderer', r)
    window.history.replaceState({}, '', url.toString())
    try {
      localStorage.setItem('stratum.renderer', r)
    } catch (_e) {
      // localStorage may be blocked (private mode) — silently degrade.
    }
  })

  const scene = createMemo(() => {
    const d = data()
    if (!d) return null
    return computeLayout(d, { density: tweaks().density, direction: tweaks().direction })
  })

  let stageRef!: HTMLDivElement

  const measureStage = () => {
    if (!stageRef) return
    const r = stageRef.getBoundingClientRect()
    setStageSize({ w: r.width, h: r.height })
    setStageRect(r)
  }

  // stageRef is bound only when <Show when={data()}> resolves and renders
  // the `.stage` div — which can happen AFTER this onMount fires. Set up
  // the observer via createEffect on data() so we wait for the ref.
  createEffect(() => {
    if (!data() || !stageRef) return
    measureStage()
    const ro = new ResizeObserver(() => measureStage())
    ro.observe(stageRef)
    onCleanup(() => ro.disconnect())
  })

  createEffect(() => {
    // re-measure when panels collapse
    leftCollapsed()
    rightCollapsed()
    queueMicrotask(measureStage)
  })

  const fitToBounds = () => {
    const sc = scene()
    const ss = stageSize()
    if (!sc || !ss.w || !sc.width) return
    const padding = 40
    const fx = (ss.w - padding * 2) / sc.width
    const fy = (ss.h - padding * 2) / sc.height
    const z = Math.max(0.15, Math.min(2, Math.min(fx, fy)))
    setViewport({
      zoom: z,
      x: (ss.w - sc.width * z) / 2,
      y: (ss.h - sc.height * z) / 2,
    })
  }

  // Auto-fit the first time scene + stage size are both available, and once
  // more whenever a brand-new scene arrives (different data → reset the flag).
  createEffect(() => {
    void scene()
    setHasFitted(false)
  })
  createEffect(() => {
    const sc = scene()
    const ss = stageSize()
    if (!sc || !ss.w || hasFitted()) return
    fitToBounds()
    setHasFitted(true)
  })

  const centerOn = (id: string) => {
    const sc = scene()
    if (!sc) return
    const pos = sc.modulePos[id]
    if (!pos) return
    const v = viewport()
    const z = Math.max(v.zoom, 0.85)
    setViewport({
      zoom: z,
      x: stageSize().w / 2 - pos.cx * z,
      y: stageSize().h / 2 - pos.cy * z,
    })
  }

  const handleSelect = (id: string) => {
    setSelected(id)
    if (id) centerOn(id)
  }

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null
      const inInput =
        target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA' || target?.isContentEditable
      if (e.key === 'Escape') {
        if (paletteOpen()) return
        if (focusedCycle()) {
          setFocusedCycle(null)
          return
        }
        if (selected()) {
          setSelected(null)
          return
        }
        fitToBounds()
      }
      if (e.key === 'f' && !e.metaKey && !e.ctrlKey && !inInput) {
        fitToBounds()
      }
      if (e.key === '[' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        setLeftCollapsed(!leftCollapsed())
      }
      if (e.key === ']' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        setRightCollapsed(!rightCollapsed())
      }
    }
    window.addEventListener('keydown', onKey)
    onCleanup(() => window.removeEventListener('keydown', onKey))
  })

  const bodyStyle = createMemo(() => {
    const leftCol = leftCollapsed() ? '0' : leftWidth() + 'px'
    const rightCol = rightCollapsed() ? '0' : rightWidth() + 'px'
    const leftHandle = leftCollapsed() ? '0' : '5px'
    const rightHandle = rightCollapsed() ? '0' : '5px'
    return {
      'grid-template-columns': `${leftCol} ${leftHandle} 1fr ${rightHandle} ${rightCol}`,
    } as Record<string, string>
  })

  const selectedMod = () =>
    selected() ? data()?.modules.find((m) => m.id === selected()) ?? null : null

  return (
    <Show when={!snapshot.error} fallback={<ErrorView err={snapshot.error as Error} />}>
      <Show
        when={data()}
        fallback={
          <div style={{ padding: '24px', color: 'var(--tx-3)' }}>Loading snapshot…</div>
        }
      >
        {(d) => (
          <div class="strat-app" data-accent={tweaks().accent}>
            <Header
              data={d()}
              filters={filters()}
              setFilters={setFilters}
              paletteOpen={paletteOpen()}
              setPaletteOpen={setPaletteOpen}
              onSelect={handleSelect}
              onResetView={fitToBounds}
              onTweaksToggle={() =>
                setTweaks({ ...tweaks(), haloThick: !tweaks().haloThick })
              }
              renderer={renderer()}
              onRendererToggle={() => setRenderer(renderer() === 'svg' ? 'webgl' : 'svg')}
            />
            <div class="strat-body" style={bodyStyle()}>
              <div class={`panel-slot ${leftCollapsed() ? 'collapsed' : ''}`}>
                <Show when={!leftCollapsed()}>
                  <OutlinePanel
                    data={d()}
                    filters={filters()}
                    setFilters={setFilters}
                    selectedId={selected()}
                    onSelect={handleSelect}
                  />
                </Show>
              </div>

              <Show when={!leftCollapsed()} fallback={<div />}>
                <ResizeHandle side="left" width={leftWidth} setWidth={setLeftWidth} />
              </Show>

              <div class="stage" ref={stageRef}>
                <Show when={scene()}>
                  {(sc) => (
                    <>
                      <Show
                        when={renderer() === 'webgl'}
                        fallback={
                          <Graph
                            scene={sc()}
                            data={d()}
                            filters={filters()}
                            tweaks={tweaks()}
                            selectedId={selected()}
                            hoveredId={hovered()?.id ?? null}
                            focusedCycle={focusedCycle()}
                            onSelect={handleSelect}
                            onHover={setHovered}
                            onHoverEdge={setHoveredEdge}
                            hoveredEdge={hoveredEdge()}
                            viewport={viewport()}
                            onViewportChange={setViewport}
                          />
                        }
                      >
                        <GraphWebGL
                          scene={sc()}
                          data={d()}
                          filters={filters()}
                          tweaks={tweaks()}
                          selectedId={selected()}
                          hoveredId={hovered()?.id ?? null}
                          focusedCycle={focusedCycle()}
                          onSelect={handleSelect}
                          onOpenCard={(id) => setCardModuleId(id)}
                          onHover={setHovered}
                          onHoverEdge={setHoveredEdge}
                          hoveredEdge={hoveredEdge()}
                          viewport={viewport()}
                          onViewportChange={setViewport}
                        />
                      </Show>
                      <Legend />
                      <Show when={focusedCycle()}>
                        {(fc) => (
                          <div class="cycle-banner">
                            <span class="cycle-dot" />
                            <span>Focused on cycle</span>
                            <code>
                              {d().violations.find((v) => v.id === fc())?.message ?? ''}
                            </code>
                            <button onClick={() => setFocusedCycle(null)}>exit ↩</button>
                          </div>
                        )}
                      </Show>
                      <ZoomControls
                        viewport={viewport()}
                        setViewport={setViewport}
                        fit={fitToBounds}
                        stageSize={stageSize()}
                      />
                      <Show when={tweaks().showTooltip}>
                        <HoverTooltip
                          data={d()}
                          hoveredModule={hovered()}
                          hoveredEdge={hoveredEdge()}
                          stageRect={stageRect()}
                        />
                      </Show>
                    </>
                  )}
                </Show>
              </div>

              <Show when={!rightCollapsed()} fallback={<div />}>
                <ResizeHandle side="right" width={rightWidth} setWidth={setRightWidth} />
              </Show>

              <div class={`panel-slot ${rightCollapsed() ? 'collapsed' : ''}`}>
                <Show when={!rightCollapsed()}>
                  <DetailsPanel
                    data={d()}
                    selectedId={selected()}
                    hoveredId={hovered()?.id ?? null}
                    onClose={() => setSelected(null)}
                    onFocusCycle={(vid) =>
                      setFocusedCycle(focusedCycle() === vid ? null : vid)
                    }
                    focusedCycle={focusedCycle()}
                    onSelect={handleSelect}
                  />
                </Show>
              </div>
            </div>
            <StatusBar
              data={d()}
              viewport={viewport()}
              selected={selectedMod()}
              leftCollapsed={leftCollapsed()}
              setLeftCollapsed={setLeftCollapsed}
              rightCollapsed={rightCollapsed()}
              setRightCollapsed={setRightCollapsed}
              filters={filters()}
            />
            <Show when={cardModuleId()}>
              {(cid) => (
                <ModuleCard
                  data={d()}
                  moduleId={cid()}
                  onClose={() => setCardModuleId(null)}
                  onSelect={handleSelect}
                  onFocusCycle={(vid) =>
                    setFocusedCycle(focusedCycle() === vid ? null : vid)
                  }
                />
              )}
            </Show>
          </div>
        )}
      </Show>
    </Show>
  )
}

const ResizeHandle: Component<{
  side: 'left' | 'right'
  width: () => number
  setWidth: (n: number) => void
}> = (props) => {
  const onMouseDown = (e: MouseEvent) => {
    e.preventDefault()
    const startX = e.clientX
    const startW = props.width()
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX
      const newW = props.side === 'left' ? startW + dx : startW - dx
      props.setWidth(Math.max(200, Math.min(560, newW)))
    }
    const onUp = () => {
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
    }
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', onUp)
  }
  return (
    <div class={`resize-handle handle-${props.side}`} onMouseDown={onMouseDown}>
      <div class="resize-handle-grip" />
    </div>
  )
}

const ErrorView: Component<{ err: Error }> = (props) => {
  const isVersionMismatch = props.err instanceof SnapshotVersionMismatchError
  return (
    <div role="alert" class="err-view">
      <strong>{isVersionMismatch ? 'Visualizer out of date' : 'Snapshot error'}</strong>
      <p>{props.err.message}</p>
    </div>
  )
}

// Pick a friendly project name from any module path — first path segment that
// looks like a real directory ("src" doesn't count).
const deriveProjectName = (snap: GraphSnapshot): string => {
  const first = snap.modules[0]?.path ?? ''
  if (!first) return 'project'
  const parts = first.split(/[/\\]/)
  for (let i = parts.length - 2; i >= 0; i--) {
    const p = parts[i]
    if (p && p !== 'src' && !p.endsWith(':') && p !== '') return p
  }
  return 'project'
}

export default App
