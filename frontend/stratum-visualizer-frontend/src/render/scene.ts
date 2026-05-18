import { Application, Container } from 'pixi.js'
import { Viewport } from 'pixi-viewport'

export interface Scene {
  app: Application
  viewport: Viewport
  nodeLayer: Container
  edgeLayer: Container
  destroy: () => void
}

export const createScene = async (host: HTMLElement): Promise<Scene> => {
  const app = new Application()
  const width = host.clientWidth || 800
  const height = host.clientHeight || 600
  await app.init({
    width,
    height,
    background: 0x1e1e1e,
    antialias: true,
    resolution: window.devicePixelRatio || 1,
    autoDensity: true,
  })
  host.appendChild(app.canvas)

  const viewport = new Viewport({
    screenWidth: width,
    screenHeight: height,
    worldWidth: width * 4,
    worldHeight: height * 4,
    events: app.renderer.events,
  })
  viewport.drag().pinch().wheel().decelerate()
  app.stage.addChild(viewport)

  const edgeLayer = new Container()
  const nodeLayer = new Container()
  viewport.addChild(edgeLayer)
  viewport.addChild(nodeLayer)

  return {
    app,
    viewport,
    nodeLayer,
    edgeLayer,
    destroy: () => {
      app.destroy(true, { children: true })
    },
  }
}
