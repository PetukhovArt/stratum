import {
  Component,
  createSignal,
  JSX,
  onCleanup,
  onMount,
  Show,
} from 'solid-js'
import { Portal } from 'solid-js/web'

interface TooltipProps {
  /** Plain-text or rich JSX shown in the tooltip body. */
  content: JSX.Element
  /** Element that triggers the tooltip on hover. */
  children: JSX.Element
  /** Delay in ms before showing — default 250. Set 0 for instant. */
  delay?: number
  /** Max width — default 280. */
  maxWidth?: number
  /** Extra CSS class for the tooltip body. */
  class?: string
  /** Optional wrapper class (applied to the inline-block span). */
  wrapClass?: string
}

interface Pos {
  left: number
  top: number
  ready: boolean
}

// Single shared host so we don't create hundreds of children at <body> for
// every tooltip site — only one tooltip is visible at a time.
let activeShowToken = 0

export const Tooltip: Component<TooltipProps> = (props) => {
  let wrapRef!: HTMLSpanElement
  let bodyRef: HTMLDivElement | undefined
  const [open, setOpen] = createSignal(false)
  const [anchor, setAnchor] = createSignal<{ x: number; y: number }>({ x: 0, y: 0 })
  const [pos, setPos] = createSignal<Pos>({ left: 0, top: 0, ready: false })
  let showTimer = 0
  let myToken = 0

  const placeTooltip = () => {
    if (!bodyRef) return
    const w = bodyRef.offsetWidth
    const h = bodyRef.offsetHeight
    const a = anchor()
    let left = a.x + 14
    let top = a.y + 14
    if (left + w > window.innerWidth - 8) left = a.x - w - 14
    if (top + h > window.innerHeight - 8) top = a.y - h - 14
    if (left < 8) left = 8
    if (top < 8) top = 8
    setPos({ left, top, ready: true })
  }

  const onEnter = (e: MouseEvent) => {
    setAnchor({ x: e.clientX, y: e.clientY })
    const d = props.delay ?? 250
    window.clearTimeout(showTimer)
    showTimer = window.setTimeout(() => {
      myToken = ++activeShowToken
      const a = anchor()
      // Seed the position near the cursor on the first paint. The CSS
      // transition on left/top would otherwise animate from (0, 0) — which
      // looks like the tooltip is "flying in" from the top-left corner of
      // the page. placeTooltip refines this on the next microtask once the
      // body has been measured.
      setPos({ left: a.x + 14, top: a.y + 14, ready: false })
      setOpen(true)
      queueMicrotask(placeTooltip)
    }, d)
  }
  const onMove = (e: MouseEvent) => {
    if (!open()) {
      setAnchor({ x: e.clientX, y: e.clientY })
      return
    }
    setAnchor({ x: e.clientX, y: e.clientY })
    queueMicrotask(placeTooltip)
  }
  const onLeave = () => {
    window.clearTimeout(showTimer)
    // Only the currently-visible tooltip should close itself.
    if (myToken === activeShowToken) setOpen(false)
    else setOpen(false)
  }

  onMount(() => {
    wrapRef.addEventListener('mouseenter', onEnter)
    wrapRef.addEventListener('mousemove', onMove)
    wrapRef.addEventListener('mouseleave', onLeave)
    onCleanup(() => {
      wrapRef.removeEventListener('mouseenter', onEnter)
      wrapRef.removeEventListener('mousemove', onMove)
      wrapRef.removeEventListener('mouseleave', onLeave)
      window.clearTimeout(showTimer)
    })
  })

  return (
    <>
      <span ref={wrapRef!} class={`tt-wrap ${props.wrapClass ?? ''}`}>
        {props.children}
      </span>
      <Show when={open()}>
        <Portal>
          <div
            ref={bodyRef}
            class={`tt tt-html ${props.class ?? ''}`}
            style={{
              position: 'fixed',
              left: pos().left + 'px',
              top: pos().top + 'px',
              'max-width': (props.maxWidth ?? 280) + 'px',
              opacity: pos().ready ? 1 : 0,
              // Suppress the .tt class's left/top transition until the body
              // has been measured and positioned — otherwise the very first
              // adjustment (anchor-estimate → viewport-clamped position)
              // animates visibly and reads as a slide.
              transition: pos().ready ? undefined : 'none',
              'pointer-events': 'none',
              'z-index': 9999,
            }}
          >
            {props.content}
          </div>
        </Portal>
      </Show>
    </>
  )
}
