import { Accessor, Component, createMemo, createSignal, JSX, onMount, Show } from 'solid-js'

const CURSOR_OFFSET_PX = 14
const VIEWPORT_MARGIN_PX = 8
const MAX_WIDTH_PX = 320

interface Point {
  x: number
  y: number
}

export type TipRender = () => JSX.Element

export interface TipController {
  show: (e: MouseEvent, render: TipRender) => void
  move: (e: MouseEvent) => void
  hide: () => void
}

export const createTip = (): {
  controller: TipController
  render: Accessor<TipRender | null>
  point: Accessor<Point>
} => {
  const [render, setRender] = createSignal<TipRender | null>(null)
  const [point, setPoint] = createSignal<Point>({ x: 0, y: 0 })
  const controller: TipController = {
    show: (e, next) => {
      setPoint({ x: e.clientX, y: e.clientY })
      setRender(() => next)
    },
    move: (e) => setPoint({ x: e.clientX, y: e.clientY }),
    hide: () => setRender(null),
  }
  return { controller, render, point }
}

const clampAxis = (cursor: number, size: number, limit: number): number => {
  const after = cursor + CURSOR_OFFSET_PX
  const placed = after + size > limit - VIEWPORT_MARGIN_PX ? cursor - size - CURSOR_OFFSET_PX : after
  return Math.max(VIEWPORT_MARGIN_PX, placed)
}

const TreeTooltipBox: Component<{ point: Point; render: TipRender }> = (props) => {
  const [size, setSize] = createSignal<{ w: number; h: number } | null>(null)
  let ref!: HTMLDivElement
  onMount(() => setSize({ w: ref.offsetWidth, h: ref.offsetHeight }))

  const position = createMemo(() => {
    const s = size()
    if (!s) return null
    return {
      left: clampAxis(props.point.x, s.w, window.innerWidth),
      top: clampAxis(props.point.y, s.h, window.innerHeight),
    }
  })

  return (
    <div
      ref={ref}
      class="tt tree-tt"
      style={{
        position: 'fixed',
        left: `${position()?.left ?? 0}px`,
        top: `${position()?.top ?? 0}px`,
        'max-width': `${MAX_WIDTH_PX}px`,
        opacity: position() ? 1 : 0,
      }}
    >
      {props.render()}
    </div>
  )
}

export const TreeTooltip: Component<{ render: TipRender | null; point: Point }> = (props) => (
  <Show when={props.render} keyed>
    {(render) => <TreeTooltipBox point={props.point} render={render} />}
  </Show>
)
