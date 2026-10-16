import { Component, createMemo, For, Show } from 'solid-js'
import { LaidEdge } from '../../render/layout'
import { EDGE_DASH, SEV_COLOR, SEVERITY_LABELS } from '../../render/encoding'
import { HoveredEdge } from '../../state'
import { DIMMED_OPACITY } from './dimming'

export type EdgeDirection = 'in' | 'out' | null

const DIRECTION_COLOR = { out: 'oklch(75% 0.16 150)', in: 'oklch(78% 0.14 200)' } as const
const KIND_COLOR = {
  static: 'oklch(72% 0.02 240)',
  di: 'oklch(72% 0.10 200)',
  runtime: 'oklch(72% 0.10 290)',
} as const
const VIOLATION_DASH = '6 4'
const CURVE_TENSION = 0.4

const curvePath = (e: LaidEdge): string => {
  const dy = e.y2 - e.y1
  return `M${e.x1},${e.y1} C${e.x1},${e.y1 + dy * CURVE_TENSION} ${e.x2},${e.y2 - dy * CURVE_TENSION} ${e.x2},${e.y2}`
}

const ArrowMarker: Component<{ id: string; size: number; color: string }> = (props) => (
  <marker
    id={props.id}
    viewBox="0 0 8 8"
    refX="7"
    refY="4"
    markerWidth={props.size}
    markerHeight={props.size}
    orient="auto"
  >
    <path d="M0,0 L8,4 L0,8 z" style={{ fill: props.color }} />
  </marker>
)

export const EdgeMarkers: Component = () => (
  <>
    <ArrowMarker id="arrow-default" size={5} color="currentColor" />
    <For each={SEVERITY_LABELS}>
      {(s) => <ArrowMarker id={`arrow-${s}`} size={6} color={SEV_COLOR[s]} />}
    </For>
    <ArrowMarker id="arrow-out" size={7} color={DIRECTION_COLOR.out} />
    <ArrowMarker id="arrow-in" size={7} color={DIRECTION_COLOR.in} />
  </>
)

export const EdgeLine: Component<{
  edge: LaidEdge
  direction: EdgeDirection
  isDimmed: boolean
  isFocused: boolean
  isHovered: boolean
  isInteractive: boolean
  onHover: (h: HoveredEdge | null) => void
}> = (props) => {
  const path = createMemo(() => curvePath(props.edge))
  const severity = () => {
    const s = props.edge.severity
    return s ? { color: SEV_COLOR[s], arrow: `arrow-${s}` } : null
  }
  const isConnected = () => props.direction !== null

  const stroke = () => {
    const s = severity()
    if (s) return s.color
    if (props.direction) return DIRECTION_COLOR[props.direction]
    return KIND_COLOR[props.edge.kind]
  }
  const opacity = () => {
    if (props.isFocused) return 1
    if (props.isDimmed) return DIMMED_OPACITY.edge
    if (severity()) return 0.95
    if (isConnected()) return 1
    if (props.isHovered) return 0.95
    return 0.3
  }
  const strokeWidth = () => {
    if (severity()) return 1.7
    if (isConnected()) return 1.6
    return props.isHovered ? 1.4 : 0.8
  }
  const marker = () => {
    const s = severity()
    if (s) return `url(#${s.arrow})`
    if (props.direction) return `url(#arrow-${props.direction})`
    return 'url(#arrow-default)'
  }

  const onEnter = (e: MouseEvent) =>
    props.onHover({
      i: props.edge.i,
      source: props.edge.source,
      target: props.edge.target,
      kind: props.edge.kind,
      violation: props.edge.violation,
      x: e.clientX,
      y: e.clientY,
    })

  return (
    <g
      data-edge-i={props.edge.i}
      style={{ color: stroke() }}
      onMouseEnter={props.isInteractive ? onEnter : undefined}
      onMouseLeave={props.isInteractive ? () => props.onHover(null) : undefined}
    >
      <Show when={props.isInteractive}>
        <path
          d={path()}
          fill="none"
          stroke="transparent"
          stroke-width={10}
          style={{ 'pointer-events': 'stroke' }}
        />
      </Show>
      <Show when={isConnected() && !severity()}>
        <path
          d={path()}
          fill="none"
          stroke-width={3.5}
          opacity={0.2}
          style={{ stroke: stroke(), 'pointer-events': 'none' }}
        />
      </Show>
      <path
        d={path()}
        fill="none"
        stroke-width={strokeWidth()}
        stroke-dasharray={EDGE_DASH[props.edge.kind] ?? (severity() ? VIOLATION_DASH : undefined)}
        opacity={opacity()}
        marker-end={marker()}
        style={{ stroke: stroke(), 'pointer-events': 'none' }}
      />
      <Show when={severity()}>
        {(s) => (
          <circle
            r={3}
            cx={(props.edge.x1 + props.edge.x2) / 2}
            cy={(props.edge.y1 + props.edge.y2) / 2}
            class="pulse-dot"
            style={{ fill: s().color, 'pointer-events': 'none' }}
          />
        )}
      </Show>
    </g>
  )
}
