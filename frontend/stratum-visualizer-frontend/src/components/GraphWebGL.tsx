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

  const initPixi = async () => {
    app = new PIXI.Application()
    await app.init({
      resizeTo: hostRef,
      backgroundAlpha: 0,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    })
    hostRef.appendChild(app.canvas)

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
    if (pixiScene) pixiScene.destroy()
    if (pxViewport) pxViewport.destroy({ children: true })
    if (app) app.destroy(true, { children: true, texture: true })
    pixiScene = null
    pxViewport = null
    app = null
  })

  return <div class="strat-graph-webgl" ref={hostRef} />
}
