import { Component } from 'solid-js'
import { LaneInfo } from '../../render/layout'
import { layerColor } from '../../render/encoding'
import { TONE } from '../../render/palette'
import { DIMMED_OPACITY } from './dimming'

export const Lane: Component<{ lane: LaneInfo; isDimmed: boolean; sceneWidth: number }> = (
  props,
) => (
  <g opacity={props.isDimmed ? DIMMED_OPACITY.lane : 1}>
    <rect
      x={0}
      y={props.lane.y}
      width={props.sceneWidth}
      height={props.lane.height}
      fill={layerColor(props.lane.hue, TONE.laneFill)}
      stroke="rgba(255,255,255,0.04)"
      stroke-width={1}
      rx={8}
    />
    <rect
      x={0}
      y={props.lane.y}
      width={4}
      height={props.lane.height}
      fill={layerColor(props.lane.hue, TONE.laneBar)}
      rx={2}
    />
    <text
      x={14}
      y={props.lane.y + 22}
      class="lane-label-text"
      fill={layerColor(props.lane.hue, TONE.laneLabel)}
    >
      {props.lane.label.toUpperCase()}
    </text>
    <text x={14} y={props.lane.y + 38} class="lane-count-text">
      {props.lane.modules} modules{' '}
      {props.lane.errors > 0 ? `· ${props.lane.errors} err` : '·'}{' '}
      {props.lane.warnings > 0 ? `· ${props.lane.warnings} warn` : ''}
    </text>
  </g>
)
