import { Component, createEffect, onCleanup, onMount } from 'solid-js'
import * as PIXI from 'pixi.js'
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

  onMount(() => {
    void initPixi()
  })

  let hostResizeObserver: ResizeObserver | null = null

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
    // Assign `app` only after init resolves so the ticker-sync poll never
    // sees a half-initialized Application (avoids the "ticker.add of
    // undefined" runaway loop).
    app = newApp
    hostRef.appendChild(app.canvas)

    hostResizeObserver = new ResizeObserver(() => {
      if (!app) return
      app.renderer.resize(hostRef.clientWidth, hostRef.clientHeight)
      if (pxViewport) {
        pxViewport.resize(hostRef.clientWidth, hostRef.clientHeight)
      }
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

    pxViewport.on('moved', () => {
      if (!pxViewport) return
      props.onViewportChange({
        x: pxViewport.x,
        y: pxViewport.y,
        zoom: pxViewport.scale.x,
      })
    })

    // Apply the current viewport prop on init. App.tsx's fit-to-bounds effect
    // may have already fired before initPixi resolved; the viewport-sync
    // createEffect early-returns while pxViewport is null, so we'd never
    // catch up otherwise.
    pxViewport.scale.set(props.viewport.zoom)
    pxViewport.position.set(props.viewport.x, props.viewport.y)

    rebuildScene()
  }

  const rebuildScene = () => {
    if (!pxViewport) return
    if (pixiScene) {
      pxViewport.removeChild(pixiScene.root)
      pixiScene.destroy()
    }
    pixiScene = buildPixiScene(props.scene)
    pxViewport.addChild(pixiScene.root)
    pxViewport.worldWidth = props.scene.width
    pxViewport.worldHeight = props.scene.height
  }

  let hitIndex: HitTestIndex | null = null
  let lastHoverId: string | null = null
  let pointerThrottle = 0

  const updateHitIndex = () => {
    hitIndex = buildHitTest(props.scene)
  }

  createEffect(() => {
    void props.scene
    updateHitIndex()
  })

  const onPointerMove = (e: PointerEvent) => {
    if (!pxViewport || !hitIndex) return
    const now = performance.now()
    if (now - pointerThrottle < 16) return
    pointerThrottle = now
    const rect = hostRef.getBoundingClientRect()
    const localX = (e.clientX - rect.left - pxViewport.x) / pxViewport.scale.x
    const localY = (e.clientY - rect.top - pxViewport.y) / pxViewport.scale.y
    const id = hitIndex.queryPoint(localX, localY)
    if (id !== lastHoverId) {
      lastHoverId = id
      props.onHover(id ? { id, x: e.clientX, y: e.clientY } : null)
    }
  }

  const onPointerClick = (e: PointerEvent) => {
    if (!pxViewport || !hitIndex) return
    const rect = hostRef.getBoundingClientRect()
    const localX = (e.clientX - rect.left - pxViewport.x) / pxViewport.scale.x
    const localY = (e.clientY - rect.top - pxViewport.y) / pxViewport.scale.y
    const id = hitIndex.queryPoint(localX, localY)
    if (id) props.onSelect(id)
  }

  onMount(() => {
    hostRef.addEventListener('pointermove', onPointerMove)
    hostRef.addEventListener('click', onPointerClick)
    onCleanup(() => {
      hostRef.removeEventListener('pointermove', onPointerMove)
      hostRef.removeEventListener('click', onPointerClick)
    })
  })

  // Rebuild whenever Scene reference changes (layout produced a new object).
  createEffect(() => {
    void props.scene
    if (pxViewport && app) rebuildScene()
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
    if (hostResizeObserver) hostResizeObserver.disconnect()
    if (pixiScene) pixiScene.destroy()
    if (pxViewport) pxViewport.destroy({ children: true })
    if (app) app.destroy(true, { children: true, texture: true })
    hostResizeObserver = null
    pixiScene = null
    pxViewport = null
    app = null
  })

  // ── Label overlay sync ────────────────────────────────────────────────
  let overlayRef!: HTMLDivElement

  const syncOverlay = () => {
    if (!pxViewport || !overlayRef) return
    const x = pxViewport.x
    const y = pxViewport.y
    const s = pxViewport.scale.x
    overlayRef.style.transform = `translate(${x}px, ${y}px) scale(${s})`
  }

  // Subscribe to every Pixi ticker frame; sync is one writeAttribute, cheap.
  // `app` is set only after `app.init()` resolves, so `app.ticker` is
  // guaranteed to exist when truthy. The poll exits the first time it sees
  // a fully-initialized Application or gives up after 5 seconds of waiting
  // (init really should not take that long; if it does, init failed and the
  // canvas won't render either way — keep noise out of the console).
  onMount(() => {
    const tick = () => syncOverlay()
    let attempts = 0
    const interval = window.setInterval(() => {
      attempts++
      if (app && app.ticker) {
        app.ticker.add(tick)
        window.clearInterval(interval)
      } else if (attempts > 312) {
        window.clearInterval(interval)
      }
    }, 16)
    onCleanup(() => {
      window.clearInterval(interval)
      if (app && app.ticker) app.ticker.remove(tick)
    })
  })

  return (
    <div class="strat-graph-webgl" ref={hostRef}>
      <div class="webgl-labels-overlay" ref={overlayRef}>
        {Object.entries(props.scene.modulePos).map(([id, p]) => (
          <div
            class="webgl-module-label"
            data-id={id}
            style={{
              transform: `translate(${p.x + 4}px, ${p.y + 2}px)`,
              'max-width': `${p.w - 8}px`,
            }}
          >
            {p.mod.label}
          </div>
        ))}
      </div>
    </div>
  )
}
