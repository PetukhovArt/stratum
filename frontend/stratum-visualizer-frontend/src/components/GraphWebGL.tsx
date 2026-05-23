import { Component, createEffect, onCleanup, onMount } from 'solid-js'
import * as PIXI from 'pixi.js'
import 'pixi.js/prepare'
import { Viewport } from 'pixi-viewport'
import type { Scene } from '../render/layout'
import { DesignData } from '../render/design'
import {
  Filters,
  HoveredEdge,
  HoveredMod,
  Tweaks,
  Viewport as ViewportState,
} from '../state'
import { buildPixiScene, PixiScene } from '../render/webgl/scene'
import { buildHitTest, HitTestIndex } from '../render/webgl/hitTest'

interface GraphWebGLProps {
  scene: Scene
  data: DesignData
  filters: Filters
  tweaks: Tweaks
  selectedId: string | null
  hoveredId: string | null
  focusedCycle: string | null
  onSelect: (id: string) => void
  /** Fired in addition to onSelect when the clicked module renders tiny on screen. */
  onOpenCard?: (id: string) => void
  onHover: (h: HoveredMod | null) => void
  onHoverEdge: (h: HoveredEdge | null) => void
  hoveredEdge: HoveredEdge | null
  viewport: ViewportState
  onViewportChange: (v: ViewportState) => void
}

export const GraphWebGL: Component<GraphWebGLProps> = (props) => {
  let hostRef!: HTMLDivElement
  let app: PIXI.Application | null = null
  let pxViewport: Viewport | null = null
  let pixiScene: PixiScene | null = null
  let hostResizeObserver: ResizeObserver | null = null
  let hitIndex: HitTestIndex | null = null
  let lastHoverId: string | null = null
  let pointerThrottle = 0
  let pendingViewportSync = false
  let lastZoomApplied = -1
  // Hover dwell debounce — see onPointerMove for rationale.
  let hoverDwellTimer: number | null = null
  let pendingPointerX = 0
  let pendingPointerY = 0

  const initPixi = async () => {
    // Pixi v8's `resizeTo: hostRef` triggers an internal ResizeObserver that
    // throws if the ref isn't a real Element at init time. Use explicit
    // width/height + our own observer instead — works reliably across the
    // Solid mount timing edge cases.
    const w = hostRef.clientWidth || 800
    const h = hostRef.clientHeight || 600
    const newApp = new PIXI.Application()
    await newApp.init({
      width: w,
      height: h,
      backgroundAlpha: 0,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    })

    // If the component unmounted while init was awaiting, drop the half-built
    // app on the floor — cleanup already nulled out everything else.
    if (!hostRef.isConnected) {
      newApp.destroy(
        { removeView: true },
        { children: true, texture: true, textureSource: true },
      )
      return
    }

    app = newApp
    hostRef.appendChild(app.canvas)

    hostResizeObserver = new ResizeObserver(() => {
      if (!app) return
      const w = hostRef.clientWidth
      const h = hostRef.clientHeight
      app.renderer.resize(w, h)
      if (pxViewport) pxViewport.resize(w, h)
    })
    hostResizeObserver.observe(hostRef)

    pxViewport = new Viewport({
      screenWidth: hostRef.clientWidth,
      screenHeight: hostRef.clientHeight,
      worldWidth: props.scene.width,
      worldHeight: props.scene.height,
      events: app.renderer.events,
    })
    pxViewport
      .drag()
      .pinch()
      .wheel({ smooth: 8 })
      .decelerate({ friction: 0.92 })
      .clampZoom({ minScale: 0.05, maxScale: 4 })

    app.stage.addChild(pxViewport)

    // Coalesce viewport state writes across rapid `moved` events. pixi-viewport
    // fires `moved` every drag tick; reading off the ticker once per frame is
    // enough to keep Solid's signal in sync without re-rendering Header /
    // StatusBar 60× per second.
    pxViewport.on('moved', () => {
      pendingViewportSync = true
    })

    app.ticker.add(() => {
      if (!pxViewport) return
      // Update label LOD only when zoom actually changes — flipping
      // .visible is O(N labels) but happens at most once per frame, and is
      // a noop while panning at a constant zoom.
      const z = pxViewport.scale.x
      if (z !== lastZoomApplied) {
        lastZoomApplied = z
        pixiScene?.setZoom(z)
      }
      if (pendingViewportSync) {
        pendingViewportSync = false
        props.onViewportChange({
          x: pxViewport.x,
          y: pxViewport.y,
          zoom: pxViewport.scale.x,
        })
      }
    })

    // Apply the current viewport prop on init. App.tsx's fit-to-bounds effect
    // may have already fired before initPixi resolved; the viewport-sync
    // createEffect early-returns while pxViewport is null, so we'd never
    // catch up otherwise.
    pxViewport.scale.set(props.viewport.zoom)
    pxViewport.position.set(props.viewport.x, props.viewport.y)

    rebuildScene()

    // Pre-upload geometry/textures so the first rendered frame doesn't hitch.
    try {
      await app.renderer.prepare.upload(app.stage)
    } catch (_e) {
      // prepare is best-effort; never block init on its failure.
    }
  }

  const rebuildScene = () => {
    if (!pxViewport || !app) return
    if (pixiScene) {
      pxViewport.removeChild(pixiScene.root)
      pixiScene.destroy()
    }
    pixiScene = buildPixiScene(props.scene, {
      filters: props.filters,
      tweaks: props.tweaks,
      highlightId: props.hoveredId ?? props.selectedId,
      focusedCycle: props.focusedCycle,
    })
    pxViewport.addChild(pixiScene.root)
    pxViewport.worldWidth = props.scene.width
    pxViewport.worldHeight = props.scene.height
    pixiScene.setZoom(pxViewport.scale.x)
    lastZoomApplied = pxViewport.scale.x
    // Re-apply current selection / hover overlays — they live on the new
    // scene's hoverG/selectG layers.
    pixiScene.setSelected(props.selectedId)
    pixiScene.setHover(props.hoveredId)
  }

  const updateHitIndex = () => {
    hitIndex = buildHitTest(props.scene)
  }

  const TINY_THRESHOLD_PX = 28
  const HOVER_DWELL_MS = 120
  // While the pointer is moving, no node is "committed" as hovered: the WebGL
  // highlight overlay and the HoverTooltip (which fans out into O(N) find +
  // O(E) edge filters per render) stay quiet. They only fire once the pointer
  // sits still for HOVER_DWELL_MS, which is what the user actually wants when
  // they stop on a node to read it. Sweeping across the graph is now free.
  const cancelHoverDwell = () => {
    if (hoverDwellTimer !== null) {
      clearTimeout(hoverDwellTimer)
      hoverDwellTimer = null
    }
  }
  const commitHoverAtPending = () => {
    hoverDwellTimer = null
    if (!pxViewport || !hitIndex) return
    const rect = hostRef.getBoundingClientRect()
    const localX = (pendingPointerX - rect.left - pxViewport.x) / pxViewport.scale.x
    const localY = (pendingPointerY - rect.top - pxViewport.y) / pxViewport.scale.y
    const id = hitIndex.queryPoint(localX, localY)
    if (id === lastHoverId) return
    lastHoverId = id
    if (id) {
      const pos = props.scene.modulePos[id]
      const tiny =
        pos !== undefined &&
        pos.leaf &&
        pos.w * pxViewport.scale.x < TINY_THRESHOLD_PX
      props.onHover({ id, x: pendingPointerX, y: pendingPointerY, tiny })
    } else {
      props.onHover(null)
    }
  }
  const onPointerMove = (e: PointerEvent) => {
    if (!pxViewport || !hitIndex) return
    const now = performance.now()
    if (now - pointerThrottle < 16) return
    pointerThrottle = now
    pendingPointerX = e.clientX
    pendingPointerY = e.clientY
    // Drop any committed hover the instant the pointer moves — the overlay
    // and tooltip should not lag behind the cursor. The replacement (if any)
    // is scheduled via the dwell timer below.
    if (lastHoverId !== null) {
      lastHoverId = null
      props.onHover(null)
    }
    cancelHoverDwell()
    hoverDwellTimer = window.setTimeout(commitHoverAtPending, HOVER_DWELL_MS)
  }
  const onPointerLeave = () => {
    cancelHoverDwell()
    if (lastHoverId !== null) {
      lastHoverId = null
      props.onHover(null)
    }
  }

  const onPointerClick = (e: PointerEvent) => {
    if (!pxViewport || !hitIndex) return
    const rect = hostRef.getBoundingClientRect()
    const localX = (e.clientX - rect.left - pxViewport.x) / pxViewport.scale.x
    const localY = (e.clientY - rect.top - pxViewport.y) / pxViewport.scale.y
    const id = hitIndex.queryPoint(localX, localY)
    if (!id) return
    props.onSelect(id)
    const pos = props.scene.modulePos[id]
    if (
      pos &&
      pos.leaf &&
      pos.w * pxViewport.scale.x < TINY_THRESHOLD_PX &&
      props.onOpenCard
    ) {
      props.onOpenCard(id)
    }
  }

  onMount(() => {
    void initPixi()
    hostRef.addEventListener('pointermove', onPointerMove, { passive: true })
    hostRef.addEventListener('pointerleave', onPointerLeave)
    hostRef.addEventListener('click', onPointerClick)
  })

  createEffect(() => {
    void props.scene
    updateHitIndex()
  })

  // Scene rebuild — keep `selectedId` out of dependencies unless we're in
  // `connectionMode === 'minimal'`, where it actually changes edge visibility.
  // Otherwise selection is a pure overlay concern and doesn't touch the
  // Pixi tree. `hoveredId` stays out of deps — 60Hz pointermove triggering
  // a full rebuild would crater perf even with batched Graphics.
  createEffect(() => {
    void props.scene
    void props.filters
    void props.tweaks
    void props.focusedCycle
    if (props.filters.connectionMode === 'minimal') void props.selectedId
    if (pxViewport && app) rebuildScene()
  })

  // Hover and selection overlays — cheap, no rebuild. setHover/setSelected
  // just clear and redraw a tiny Graphics; ticker keeps the rest cached.
  createEffect(() => {
    const id = props.hoveredId
    if (pixiScene) pixiScene.setHover(id)
  })
  createEffect(() => {
    const id = props.selectedId
    if (pixiScene) pixiScene.setSelected(id)
  })

  // External viewport changes (e.g. fit-to-bounds from App.tsx).
  createEffect(() => {
    const v = props.viewport
    if (!pxViewport) return
    if (pxViewport.x === v.x && pxViewport.y === v.y && pxViewport.scale.x === v.zoom) return
    pxViewport.scale.set(v.zoom)
    pxViewport.position.set(v.x, v.y)
  })

  onCleanup(() => {
    hostRef?.removeEventListener('pointermove', onPointerMove)
    hostRef?.removeEventListener('pointerleave', onPointerLeave)
    hostRef?.removeEventListener('click', onPointerClick)
    cancelHoverDwell()
    if (hostResizeObserver) hostResizeObserver.disconnect()
    // app.destroy(true, ...) tears down the renderer, stage, and every
    // descendant including pxViewport + pixiScene. Calling pxViewport.destroy
    // first would double-destroy. `releaseGlobalResources` flushes pooled
    // batches so a subsequent SVG↔WebGL renderer toggle doesn't reuse stale
    // GPU state.
    if (app) {
      app.destroy(
        { removeView: true, releaseGlobalResources: true },
        { children: true, texture: true, textureSource: true },
      )
    }
    hostResizeObserver = null
    pixiScene = null
    pxViewport = null
    app = null
    hitIndex = null
  })

  return <div class="strat-graph-webgl" ref={hostRef} />
}
