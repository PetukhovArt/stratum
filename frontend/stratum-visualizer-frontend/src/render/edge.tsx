import { Component, JSX, Show } from 'solid-js'
import { CANVAS_DEFS, EDGE_DASH, EDGE_NEUTRAL, EdgeKindLabel, SEV_COLOR, SeverityLabel } from './encoding'
import { Point } from './route'
import './edge.css'

export type EdgeKind = 'neutral' | 'break' | 'flow'
export type EdgeState = 'idle' | 'focused' | 'selected' | 'dimmed'
export type DataAttrs = Readonly<Record<`data-${string}`, string | number | undefined>>

export interface EdgeLabel extends Point {
  text: string | number
  class?: string
  color?: string
}

export interface EdgeProps {
  d: string
  kind?: EdgeKind
  severity?: SeverityLabel | null
  imports?: EdgeKindLabel
  dash?: string
  color?: string
  weight?: number
  width?: number
  state?: EdgeState
  opacity?: number
  label?: EdgeLabel | null
  marker?: string | null
  testId?: string
  class?: string
  data?: DataAttrs
  pathData?: DataAttrs
  title?: JSX.Element
  children?: JSX.Element
  onClick?: (e: MouseEvent) => void
  onHover?: (isOver: boolean) => void
  hit?: boolean
}

export const EDGE_SELECTED = 'oklch(94% 0.005 260)'
const LABEL_FILL = 'oklch(92% 0.005 260)'
const LABEL_HALO = 'oklch(16% 0.012 260)'
const HIT_WIDTH = 14
const BASE_WIDTH = 1.2
const STATE_WIDTH: Readonly<Record<EdgeState, number>> = { idle: 0, focused: 0.4, selected: 1.2, dimmed: 0 }
const STATE_OPACITY: Readonly<Record<Exclude<EdgeState, 'idle'>, number>> = { focused: 0.9, selected: 0.95, dimmed: 0.15 }
const IDLE_OPACITY = { neutral: 0.5, break: 0.8 } as const
const LABEL_DIMMED = 0.45

const edgeWidth = (weight: number | undefined): number =>
  weight === undefined ? BASE_WIDTH : 1 + Math.min(3, Math.log2(weight + 1) * 0.5)

export const Edge: Component<EdgeProps> = (props) => {
  const state = () => props.state ?? 'idle'
  const isBreak = () => props.kind === 'break'
  const channel = () => (isBreak() ? 'sev' : 'edge')
  const stroke = () => {
    if (isBreak()) return SEV_COLOR[props.severity ?? 'error']
    return state() === 'selected' ? EDGE_SELECTED : (props.color ?? EDGE_NEUTRAL)
  }
  const dash = () => props.dash ?? (props.kind === 'flow' ? EDGE_DASH.di : props.imports ? EDGE_DASH[props.imports] : undefined)
  const width = () => (props.width ?? edgeWidth(props.weight)) + STATE_WIDTH[state()]
  const opacity = () => {
    const s = state()
    if (s !== 'idle') return STATE_OPACITY[s]
    return props.opacity ?? (isBreak() ? IDLE_OPACITY.break : IDLE_OPACITY.neutral)
  }
  const isInteractive = () => props.onClick !== undefined || props.onHover !== undefined
  return (
    <g
      {...props.data}
      class={['edge', props.class, isInteractive() ? 'edge-on' : undefined].filter(Boolean).join(' ')}
      data-testid={props.testId}
      data-state={state()}
      onClick={(e) => props.onClick?.(e)}
      onMouseEnter={() => props.onHover?.(true)}
      onMouseLeave={() => props.onHover?.(false)}
    >
      <Show when={isInteractive() && props.hit !== false}>
        <path class="edge-hit" d={props.d} fill="none" stroke="transparent" stroke-width={HIT_WIDTH} />
      </Show>
      <path
        {...props.pathData}
        class="edge-path"
        d={props.d}
        fill="none"
        data-ch={channel()}
        stroke={stroke()}
        stroke-opacity={opacity()}
        stroke-width={width()}
        stroke-dasharray={dash()}
        marker-end={props.marker === null ? undefined : (props.marker ?? CANVAS_DEFS.arrow)}
      />
      <Show when={props.label}>
        {(label) => (
          <text
            class={label().class ?? 'edge-count'}
            data-ch={channel()}
            x={label().x}
            y={label().y}
            fill={label().color ?? LABEL_FILL}
            stroke={LABEL_HALO}
            opacity={state() === 'dimmed' ? LABEL_DIMMED : 1}
          >
            {label().text}
          </text>
        )}
      </Show>
      {props.children}
      <Show when={props.title}>
        <title>{props.title}</title>
      </Show>
    </g>
  )
}
