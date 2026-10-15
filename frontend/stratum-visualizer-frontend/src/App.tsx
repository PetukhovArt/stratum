import {
  Component,
  createMemo,
  createResource,
  createSignal,
  onCleanup,
  onMount,
  Show,
} from 'solid-js'
import './styles.css'

import { DetailsPanel } from './components/details'
import { Graph } from './components/graph/graph'
import { Header } from './components/Header'
import { HoverTooltip } from './components/hover-tooltip'
import { Minimap } from './components/minimap'
import { OutlinePanel } from './components/outline/outline-panel'
import { Legend, StatusBar, ZoomControls } from './components/StatusBar'
import { bindKeymap } from './lib/keymap'
import { loadSnapshot, SnapshotVersionMismatchError } from './lib/snapshot'
import { loadViolations } from './lib/violations'
import { adaptSnapshot } from './render/adapt'
import { DesignData } from './render/design'
import { computeLayout } from './render/layout'
import { centerOn, DEFAULT_VIEWPORT, fitViewport } from './render/viewport'
import { CanvasSize, ColorMode, DEFAULT_FILTERS, Filters, HoveredEdge, HoveredMod, Viewport } from './state'

const PANEL = {
  leftWidth: 264,
  rightWidth: 340,
  minWidth: 200,
  maxWidth: 560,
  handleWidth: '5px',
} as const

const sameSize = (a: CanvasSize, b: CanvasSize): boolean => a.w === b.w && a.h === b.h

const App: Component = () => {
  const [snapshot] = createResource(() => loadSnapshot('/api/snapshot'))
  const [violations] = createResource(() => loadViolations('/api/violations'))
  const isViolationsUnavailable = () => violations.state === 'errored'

  const data = createMemo<DesignData | null>(() => {
    const s = snapshot.state === 'ready' ? snapshot() : undefined
    const v = isViolationsUnavailable() ? [] : violations.state === 'ready' ? violations() : undefined
    if (!s || !v) return null
    return adaptSnapshot(s, v)
  })
  const scene = createMemo(() => {
    const d = data()
    return d ? computeLayout(d) : null
  })

  const [filters, setFilters] = createSignal<Filters>(DEFAULT_FILTERS())
  const [colorMode, setColorMode] = createSignal<ColorMode>('layer')
  const [isHaloThick, setHaloThick] = createSignal(true)
  const [selected, setSelected] = createSignal<string | null>(null)
  const [hovered, setHovered] = createSignal<HoveredMod | null>(null)
  const [hoveredEdge, setHoveredEdge] = createSignal<HoveredEdge | null>(null)
  const [isPaletteOpen, setPaletteOpen] = createSignal(false)
  const [focusedCycle, setFocusedCycle] = createSignal<string | null>(null)
  const [isLeftCollapsed, setLeftCollapsed] = createSignal(false)
  const [isRightCollapsed, setRightCollapsed] = createSignal(false)
  const [leftWidth, setLeftWidth] = createSignal<number>(PANEL.leftWidth)
  const [rightWidth, setRightWidth] = createSignal<number>(PANEL.rightWidth)
  const [canvasRect, setCanvasRect] = createSignal<DOMRect | null>(null)
  const [userViewport, setUserViewport] = createSignal<Viewport | null>(null)

  const canvasSize = createMemo<CanvasSize>(
    () => {
      const r = canvasRect()
      return r ? { w: r.width, h: r.height } : { w: 0, h: 0 }
    },
    { w: 0, h: 0 },
    { equals: sameSize },
  )

  const viewport = createMemo<Viewport>(() => {
    const user = userViewport()
    if (user) return user
    const sc = scene()
    return sc ? fitViewport(sc, canvasSize()) : DEFAULT_VIEWPORT
  })

  const observeCanvas = (el: HTMLDivElement) => {
    const ro = new ResizeObserver(() => setCanvasRect(el.getBoundingClientRect()))
    ro.observe(el)
    onCleanup(() => ro.disconnect())
  }

  const fit = () => setUserViewport(null)

  const handleSelect = (id: string) => {
    setSelected(id)
    const pos = scene()?.modulePos[id]
    if (pos) {
      setUserViewport(centerOn(viewport(), { x: pos.cx, y: pos.cy }, canvasSize()))
    }
  }

  const dismiss = () => {
    if (focusedCycle()) {
      setFocusedCycle(null)
      return
    }
    if (selected()) {
      setSelected(null)
      return
    }
    fit()
  }

  onMount(() => {
    const unbind = bindKeymap({
      isPaletteOpen,
      setPaletteOpen,
      dismiss,
      fit,
      toggleLeftPanel: () => setLeftCollapsed(!isLeftCollapsed()),
      toggleRightPanel: () => setRightCollapsed(!isRightCollapsed()),
    })
    onCleanup(unbind)
  })

  const bodyStyle = () => {
    const column = (isCollapsed: boolean, width: number) => (isCollapsed ? '0' : `${width}px`)
    const handle = (isCollapsed: boolean) => (isCollapsed ? '0' : PANEL.handleWidth)
    return {
      'grid-template-columns': [
        column(isLeftCollapsed(), leftWidth()),
        handle(isLeftCollapsed()),
        '1fr',
        handle(isRightCollapsed()),
        column(isRightCollapsed(), rightWidth()),
      ].join(' '),
    }
  }

  const selectedMod = () => {
    const id = selected()
    return id ? data()?.moduleById.get(id) ?? null : null
  }

  return (
    <Show when={!snapshot.error} fallback={<ErrorView err={snapshot.error as Error} />}>
      <Show
        when={data()}
        fallback={<div style={{ padding: '24px', color: 'var(--tx-3)' }}>Loading snapshot…</div>}
      >
        {(d) => (
          <div class="strat-app">
            <Header
              data={d()}
              filters={filters()}
              setFilters={setFilters}
              colorMode={colorMode()}
              setColorMode={setColorMode}
              isPaletteOpen={isPaletteOpen()}
              setPaletteOpen={setPaletteOpen}
              onSelect={handleSelect}
              onResetView={fit}
              onHaloToggle={() => setHaloThick(!isHaloThick())}
            />
            <div class="strat-body" style={bodyStyle()}>
              <div class={`panel-slot ${isLeftCollapsed() ? 'collapsed' : ''}`}>
                <Show when={!isLeftCollapsed()}>
                  <OutlinePanel
                    data={d()}
                    filters={filters()}
                    setFilters={setFilters}
                    selectedId={selected()}
                    onSelect={handleSelect}
                  />
                </Show>
              </div>

              <Show when={!isLeftCollapsed()} fallback={<div />}>
                <ResizeHandle side="left" width={leftWidth} setWidth={setLeftWidth} />
              </Show>

              <div class="canvas-area" ref={observeCanvas}>
                <Show when={scene()}>
                  {(sc) => (
                    <>
                      <Graph
                        scene={sc()}
                        data={d()}
                        filters={filters()}
                        colorMode={colorMode()}
                        isHaloThick={isHaloThick()}
                        selectedId={selected()}
                        hoveredId={hovered()?.id ?? null}
                        focusedCycle={focusedCycle()}
                        onSelect={handleSelect}
                        onHover={setHovered}
                        onHoverEdge={setHoveredEdge}
                        hoveredEdge={hoveredEdge()}
                        viewport={viewport()}
                        canvasSize={canvasSize()}
                        onViewportChange={setUserViewport}
                      />
                      <Legend />
                      <Show when={focusedCycle()}>
                        {(fc) => (
                          <div class="cycle-banner">
                            <span class="cycle-dot" />
                            <span>Focused on cycle</span>
                            <code>{d().violationById.get(fc())?.message ?? ''}</code>
                            <button onClick={() => setFocusedCycle(null)}>exit ↩</button>
                          </div>
                        )}
                      </Show>
                      <Minimap
                        scene={sc()}
                        viewport={viewport()}
                        onViewportChange={setUserViewport}
                        canvasSize={canvasSize()}
                      />
                      <ZoomControls
                        viewport={viewport()}
                        setViewport={setUserViewport}
                        fit={fit}
                        canvasSize={canvasSize()}
                      />
                      <HoverTooltip
                        data={d()}
                        hoveredModule={hovered()}
                        hoveredEdge={hoveredEdge()}
                        canvasRect={canvasRect()}
                      />
                    </>
                  )}
                </Show>
              </div>

              <Show when={!isRightCollapsed()} fallback={<div />}>
                <ResizeHandle side="right" width={rightWidth} setWidth={setRightWidth} />
              </Show>

              <div class={`panel-slot ${isRightCollapsed() ? 'collapsed' : ''}`}>
                <Show when={!isRightCollapsed()}>
                  <DetailsPanel
                    data={d()}
                    selectedId={selected()}
                    hoveredId={hovered()?.id ?? null}
                    onClose={() => setSelected(null)}
                    onFocusCycle={(vid) => setFocusedCycle(focusedCycle() === vid ? null : vid)}
                    focusedCycle={focusedCycle()}
                    onSelect={handleSelect}
                    isViolationsUnavailable={isViolationsUnavailable()}
                  />
                </Show>
              </div>
            </div>
            <StatusBar
              data={d()}
              viewport={viewport()}
              selected={selectedMod()}
              isLeftCollapsed={isLeftCollapsed()}
              setLeftCollapsed={setLeftCollapsed}
              isRightCollapsed={isRightCollapsed()}
              setRightCollapsed={setRightCollapsed}
              filters={filters()}
            />
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
  let stopDrag: (() => void) | null = null
  onCleanup(() => stopDrag?.())

  const onMouseDown = (e: MouseEvent) => {
    e.preventDefault()
    const startX = e.clientX
    const startW = props.width()
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX
      const next = props.side === 'left' ? startW + dx : startW - dx
      props.setWidth(Math.max(PANEL.minWidth, Math.min(PANEL.maxWidth, next)))
    }
    const onUp = () => {
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
      stopDrag = null
    }
    stopDrag = onUp
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

export default App
