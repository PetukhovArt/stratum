import { Component, createMemo, For } from 'solid-js'
import { Scene } from '../render/layout'
import { layerColor, SEVERITY, TONE } from '../render/palette'
import { CanvasSize, Viewport } from '../state'

const MINIMAP_W = 200
const MINIMAP_H = 140

export const Minimap: Component<{
  scene: Scene
  viewport: Viewport
  onViewportChange: (v: Viewport) => void
  canvasSize: CanvasSize
}> = (props) => {
  const scale = () =>
    Math.min(MINIMAP_W / Math.max(props.scene.width, 1), MINIMAP_H / Math.max(props.scene.height, 1))
  const offsetX = () => (MINIMAP_W - props.scene.width * scale()) / 2
  const offsetY = () => (MINIMAP_H - props.scene.height * scale()) / 2
  const violating = createMemo(() =>
    Object.values(props.scene.modulePos).filter((p) => p.mod.severity !== null),
  )

  const onClick = (e: MouseEvent) => {
    const r = (e.currentTarget as Element).getBoundingClientRect()
    const worldX = (e.clientX - r.left - offsetX()) / scale()
    const worldY = (e.clientY - r.top - offsetY()) / scale()
    props.onViewportChange({
      ...props.viewport,
      x: -worldX * props.viewport.zoom + props.canvasSize.w / 2,
      y: -worldY * props.viewport.zoom + props.canvasSize.h / 2,
    })
  }

  return (
    <div class="minimap">
      <svg width={MINIMAP_W} height={MINIMAP_H} onClick={onClick}>
        <rect x={0} y={0} width={MINIMAP_W} height={MINIMAP_H} fill="rgba(0,0,0,0.4)" rx={6} />
        <g transform={`translate(${offsetX()}, ${offsetY()}) scale(${scale()})`}>
          <For each={props.scene.lanes}>
            {(lane) => (
              <rect
                x={0}
                y={lane.y}
                width={props.scene.width}
                height={lane.height}
                fill={layerColor(lane.hue, TONE.minimapLane)}
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
          <For each={violating()}>
            {(p) => (
              <rect
                x={p.x}
                y={p.y}
                width={p.w}
                height={p.h}
                style={{ fill: p.mod.severity ? SEVERITY[p.mod.severity].color : undefined }}
              />
            )}
          </For>
        </g>
        <rect
          x={offsetX() + (-props.viewport.x / props.viewport.zoom) * scale()}
          y={offsetY() + (-props.viewport.y / props.viewport.zoom) * scale()}
          width={(props.canvasSize.w / props.viewport.zoom) * scale()}
          height={(props.canvasSize.h / props.viewport.zoom) * scale()}
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
