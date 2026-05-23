import * as PIXI from 'pixi.js'
import type { Scene } from '../layout'
import {
  CONTAINER_FILL,
  LANE_FILL,
  MODULE_FILL,
  MODULE_STROKE,
  severityFill,
  Severity,
  layerHueToHex,
} from './styles'

export interface PixiScene {
  root: PIXI.Container
  lanesLayer: PIXI.Container
  containersLayer: PIXI.Container
  modulesLayer: PIXI.Container
  edgesLayer: PIXI.Container
  destroy(): void
}

export const buildPixiScene = (scene: Scene): PixiScene => {
  const root = new PIXI.Container()
  const lanesLayer = new PIXI.Container()
  const containersLayer = new PIXI.Container()
  const modulesLayer = new PIXI.Container()
  const edgesLayer = new PIXI.Container()
  root.addChild(lanesLayer, containersLayer, modulesLayer, edgesLayer)

  // Lanes
  for (const l of scene.lanes) {
    const g = new PIXI.Graphics()
    g.rect(l.x, l.y, l.width, l.height)
    g.fill({ color: LANE_FILL, alpha: 0.4 })
    g.stroke({ color: layerHueToHex(l.hue, 70, 0.04), width: 1, alpha: 0.6 })
    lanesLayer.addChild(g)
  }

  // Containers
  for (const c of scene.containers) {
    const g = new PIXI.Graphics()
    g.roundRect(c.absX, c.absY, c.width, c.height, 6)
    g.fill({ color: CONTAINER_FILL, alpha: 0.65 })
    g.stroke({ color: layerHueToHex(parseHue(c.layer), 60, 0.08), width: 1.2, alpha: 0.8 })
    containersLayer.addChild(g)
  }

  // Modules
  for (const [id, p] of Object.entries(scene.modulePos)) {
    const g = new PIXI.Graphics()
    const sev = (p.mod.severity ?? null) as Severity | null
    const fill = sev ? severityFill(sev) : MODULE_FILL
    const alpha = sev ? 0.18 : 0.55
    g.roundRect(p.x, p.y, p.w, p.h, p.leaf ? 2 : 4)
    g.fill({ color: fill, alpha })
    g.stroke({
      color: sev ? severityFill(sev) : MODULE_STROKE,
      width: sev ? 1.6 : 1,
      alpha: sev ? 0.9 : 0.5,
    })
    g.label = id
    modulesLayer.addChild(g)
  }

  // Edges
  for (const e of scene.edges) {
    const g = new PIXI.Graphics()
    const color = e.violation ? severityFill('error') : MODULE_STROKE
    const alpha = e.violation ? 0.85 : 0.25
    g.moveTo(e.x1, e.y1)
    g.lineTo(e.x2, e.y2)
    g.stroke({ color, width: e.violation ? 1.6 : 0.8, alpha })
    edgesLayer.addChild(g)
  }

  return {
    root,
    lanesLayer,
    containersLayer,
    modulesLayer,
    edgesLayer,
    destroy() {
      root.destroy({ children: true })
    },
  }
}

// Layer ids look like "app" / "shared" / "core". Convert to a deterministic
// hue 0..360 — same algorithm as src/render/adapt.ts uses.
const parseHue = (layer: string): number => {
  let h = 0
  for (let i = 0; i < layer.length; i++) h = (h * 31 + layer.charCodeAt(i)) >>> 0
  return h % 360
}
