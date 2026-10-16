import { Component, Show } from 'solid-js'
import { ModulePos } from '../../render/layout'
import { layerColor, SeverityLabel, SEV_COLOR } from '../../render/encoding'
import { DEFAULT_HUE, facetInfo, TONE } from '../../render/palette'
import { ColorMode, HoveredMod } from '../../state'
import { CountBadge } from './count-badge'
import { DIMMED_OPACITY } from './dimming'

const SELECTED_STROKE = 'oklch(85% 0.16 65)'
const SHARED_STROKE = 'oklch(72% 0.10 290 / .55)'
const SHARED_DOT = 'oklch(78% 0.10 290 / .85)'
const SEV_GLOW: Record<SeverityLabel, string> = {
  error: 'var(--err-glow)',
  warning: 'var(--warn-glow)',
  info: 'var(--info-glow)',
}

export const ModuleNode: Component<{
  id: string
  pos: ModulePos
  hue: number | undefined
  isDimmed: boolean
  isHighlighted: boolean
  isSelected: boolean
  isHaloThick: boolean
  colorMode: ColorMode
  onSelect: (id: string) => void
  onHover: (h: HoveredMod | null) => void
}> = (props) => {
  const mod = () => props.pos.mod
  const isCompound = () => !props.pos.isLeaf
  const hue = () => props.hue ?? DEFAULT_HUE
  const isComposer = () => mod().kind === 'composer'
  const isShared = () => mod().kind === 'shared'
  const severity = () => {
    const s = mod().severity
    return s ? { color: SEV_COLOR[s], glow: SEV_GLOW[s] } : null
  }

  const stroke = () => {
    if (props.isSelected) return SELECTED_STROKE
    const s = severity()
    if (s) return s.color
    if (isComposer()) return layerColor(hue(), TONE.composerStroke)
    if (isShared()) return SHARED_STROKE
    return layerColor(hue(), TONE.moduleStroke)
  }
  const strokeWidth = () => {
    if (props.isSelected) return 2.2
    if (severity()) return 1.5
    return isComposer() ? 1.4 : 0.7
  }
  const facet = () => facetInfo(mod().facet)
  const isFacetColored = () => props.colorMode === mod().facet.axis
  const fill = () => {
    if (isFacetColored()) {
      return `color-mix(in oklch, ${facet().color} ${isCompound() ? 22 : 55}%, var(--bg-0))`
    }
    if (isCompound()) return layerColor(hue(), TONE.compoundFill)
    if (isShared()) return 'url(#shared-hatch)'
    return layerColor(hue(), TONE.leafFill)
  }
  const rounding = () => (props.pos.depth === 0 ? 6 : 4)

  return (
    <g
      transform={`translate(${props.pos.x}, ${props.pos.y})`}
      data-mod-id={props.id}
      opacity={props.isDimmed ? DIMMED_OPACITY.module : 1}
      style={{ cursor: 'pointer' }}
      onMouseEnter={(e) => props.onHover({ id: props.id, x: e.clientX, y: e.clientY })}
      onMouseLeave={() => props.onHover(null)}
      onClick={(e) => {
        e.stopPropagation()
        props.onSelect(props.id)
      }}
    >
      <Show when={isCompound() && props.isHaloThick}>
        <rect
          x={-3}
          y={-3}
          width={props.pos.w + 6}
          height={props.pos.h + 6}
          rx={rounding() + 3}
          fill="none"
          stroke={layerColor(hue(), TONE.halo)}
          stroke-width={1}
          stroke-dasharray="2 3"
        />
      </Show>
      <Show when={severity()}>
        {(s) => (
          <rect
            x={-2}
            y={-2}
            width={props.pos.w + 4}
            height={props.pos.h + 4}
            rx={rounding() + 2}
            style={{ fill: s().glow }}
            opacity={0.55}
          />
        )}
      </Show>
      <rect
        x={0}
        y={0}
        width={props.pos.w}
        height={props.pos.h}
        rx={rounding()}
        style={{ fill: fill(), stroke: stroke() }}
        stroke-width={strokeWidth()}
        stroke-dasharray={isComposer() ? '3 2' : undefined}
      />
      <Show when={isComposer() && !isCompound()}>
        <path
          d={`M ${props.pos.w / 2} 1 L ${props.pos.w - 1} ${props.pos.h / 2} L ${props.pos.w / 2} ${props.pos.h - 1} L 1 ${props.pos.h / 2} z`}
          fill="none"
          stroke={layerColor(hue(), TONE.composerGlyph)}
          stroke-width={1}
        />
      </Show>
      <Show when={isShared() && !isCompound()}>
        <circle cx={props.pos.w - 3} cy={3} r={1.8} fill={SHARED_DOT} />
      </Show>
      <Show when={props.isHighlighted}>
        <rect
          x={-3}
          y={-3}
          width={props.pos.w + 6}
          height={props.pos.h + 6}
          rx={rounding() + 3}
          fill="none"
          stroke={SELECTED_STROKE}
          stroke-width={1.4}
          stroke-dasharray="2 3"
        />
      </Show>

      <Show
        when={isCompound()}
        fallback={
          <circle
            cx={props.pos.w - 3}
            cy={props.pos.h - 3}
            r={1.4}
            style={{ fill: facet().color }}
          />
        }
      >
        <text x={7} y={12} class={`mod-label depth-${props.pos.depth} kind-${mod().kind}`}>
          {isComposer() ? '◇ ' : ''}
          {mod().label}
        </text>
        <Show when={props.pos.depth === 0}>
          <CountBadge
            errors={mod().descError}
            warnings={mod().descWarn}
            right={props.pos.w - 6}
            baselineY={12}
            hitWidth={44}
            class="mod-count"
            titleSuffix=" in descendants"
          />
        </Show>
      </Show>
    </g>
  )
}
