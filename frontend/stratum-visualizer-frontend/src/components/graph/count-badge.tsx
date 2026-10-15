import { Component, Show } from 'solid-js'
import { SEVERITY } from '../../render/palette'

const plural = (n: number, word: string): string => `${n} ${word}${n === 1 ? '' : 's'}`

export const CountBadge: Component<{
  errors: number
  warnings: number
  right: number
  baselineY: number
  hitWidth: number
  class: string
  titleSuffix?: string
}> = (props) => {
  const title = () =>
    [
      props.errors > 0 ? plural(props.errors, 'error') : '',
      props.warnings > 0 ? plural(props.warnings, 'warning') : '',
    ]
      .filter(Boolean)
      .map((line) => line + (props.titleSuffix ?? ''))
      .join('\n')

  return (
    <Show when={props.errors > 0 || props.warnings > 0}>
      <g>
        <title>{title()}</title>
        <rect
          x={props.right - props.hitWidth}
          y={props.baselineY - 11}
          width={props.hitWidth}
          height={14}
          fill="transparent"
        />
        <text x={props.right} y={props.baselineY} class={props.class} text-anchor="end">
          <Show when={props.errors > 0}>
            <tspan style={{ fill: SEVERITY.error.color }}>{props.errors}e </tspan>
          </Show>
          <Show when={props.warnings > 0}>
            <tspan style={{ fill: SEVERITY.warning.color }}>{props.warnings}w</tspan>
          </Show>
        </text>
      </g>
    </Show>
  )
}
