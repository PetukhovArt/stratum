import { Component } from 'solid-js'
import { ContainerInfo } from '../../render/layout'
import { layerColor } from '../../render/encoding'
import { DEFAULT_HUE, stageInfo, TONE } from '../../render/palette'
import { CountBadge } from './count-badge'
import { DIMMED_OPACITY } from './dimming'

export const ContainerBox: Component<{
  container: ContainerInfo
  hue: number | undefined
  isDimmed: boolean
}> = (props) => {
  const c = () => props.container
  const stage = () => stageInfo(c().stage)

  return (
    <g
      transform={`translate(${c().absX}, ${c().absY})`}
      opacity={props.isDimmed ? DIMMED_OPACITY.container : 1}
    >
      <rect
        x={0}
        y={0}
        width={c().width}
        height={c().height}
        rx={10}
        fill={layerColor(props.hue ?? DEFAULT_HUE, TONE.containerFill)}
        stroke="rgba(255,255,255,0.08)"
        stroke-width={0.8}
      />
      <g>
        <title>{stage().desc || `Stage ${c().stage}`}</title>
        <rect
          x={8}
          y={5}
          width={14}
          height={11}
          rx={3}
          style={{ fill: stage().color, stroke: stage().color }}
          fill-opacity={0.18}
          stroke-opacity={0.4}
          stroke-width={0.5}
        />
        <text
          x={15}
          y={13}
          class="container-stage-num"
          style={{ fill: stage().color }}
          text-anchor="middle"
        >
          {c().stage}
        </text>
        <rect x={6} y={3} width={18} height={15} fill="transparent" />
      </g>
      <text x={28} y={14} class="container-label">
        {c().label}
      </text>
      <CountBadge
        errors={c().errors}
        warnings={c().warnings}
        right={c().width - 10}
        baselineY={14}
        hitWidth={50}
        class="container-count"
      />
    </g>
  )
}
