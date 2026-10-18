import {
  Accessor,
  Component,
  createContext,
  createMemo,
  createSignal,
  JSX,
  onCleanup,
  onMount,
  Show,
  useContext,
} from 'solid-js'
import { t } from '../i18n'
import { CANVAS_DEFS, HATCH_STROKE } from '../render/encoding'
import { Bounds, DEFAULT_VIEWPORT, fitViewport, NO_BOX, zoomAround } from '../render/viewport'
import { CanvasSize, Viewport } from '../state'
import { activeCanvas, registerCanvas } from './active'

const WHEEL_ZOOM_FACTOR = 1.12
const BUTTON_ZOOM_FACTOR = 1.25
const DRAG_THRESHOLD_PX = 3
const EXPORT_PADDING = 24
const EXPORT_BACKGROUND = 'oklch(16% 0.012 260)'
const SVG_NS = 'http://www.w3.org/2000/svg'

export interface CanvasApi {
  viewport: Accessor<Viewport>
  setViewport: (v: Viewport) => void
  fit: () => void
  fitTo: (bounds: Bounds) => void
  rect: Accessor<DOMRect | null>
}

const CanvasContext = createContext<CanvasApi>()

export const useCanvas = (): CanvasApi => {
  const api = useContext(CanvasContext)
  if (!api) throw new Error('useCanvas() outside <Canvas>')
  return api
}

const sizeOf = (rect: DOMRect | null): CanvasSize => (rect ? { w: rect.width, h: rect.height } : { w: 0, h: 0 })
const sameSize = (a: CanvasSize, b: CanvasSize): boolean => a.w === b.w && a.h === b.h

export const Canvas: Component<{
  bounds: Bounds
  fitKey: unknown
  legend?: JSX.Element
  overlay?: JSX.Element
  children: JSX.Element
}> = (props) => {
  let wrap!: HTMLDivElement
  let svg!: SVGSVGElement
  let legendBox: HTMLDivElement | undefined
  const [rect, setRect] = createSignal<DOMRect | null>(null)
  const size = createMemo(() => sizeOf(rect()), NO_BOX, { equals: sameSize })
  const [legendCorner, setLegendCorner] = createSignal<CanvasSize>(NO_BOX, { equals: sameSize })
  const [userViewport, setUserViewport] = createSignal<{ key: unknown; viewport: Viewport } | null>(null)

  const fitOf = (bounds: Bounds) => fitViewport(bounds, size(), legendCorner())
  const viewport = createMemo<Viewport>(() => {
    const user = userViewport()
    if (user && user.key === props.fitKey) return user.viewport
    return size().w ? fitOf(props.bounds) : DEFAULT_VIEWPORT
  })
  const setViewport = (v: Viewport) => setUserViewport({ key: props.fitKey, viewport: v })
  const fit = () => setUserViewport(null)
  const fitTo = (bounds: Bounds) => setViewport(fitOf(bounds))
  const measure = () => {
    const outer = wrap.getBoundingClientRect()
    setRect(outer)
    const box = legendBox?.getBoundingClientRect()
    setLegendCorner(box?.width ? { w: box.right - outer.left, h: outer.bottom - box.top } : NO_BOX)
  }
  const zoomAtCenter = (factor: number) =>
    setViewport(zoomAround(viewport(), { x: size().w / 2, y: size().h / 2 }, factor))

  let drag: { x: number; y: number; from: Viewport } | null = null
  let hasMoved = false

  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return
    drag = { x: e.clientX, y: e.clientY, from: viewport() }
    hasMoved = false
  }
  const onPointerMove = (e: PointerEvent) => {
    if (!drag) return
    const dx = e.clientX - drag.x
    const dy = e.clientY - drag.y
    if (Math.abs(dx) + Math.abs(dy) > DRAG_THRESHOLD_PX) hasMoved = true
    if (hasMoved) setViewport({ ...drag.from, x: drag.from.x + dx, y: drag.from.y + dy })
  }
  const endDrag = () => {
    drag = null
  }
  const swallowClickAfterDrag = (e: MouseEvent) => {
    if (!hasMoved) return
    e.stopPropagation()
    hasMoved = false
  }
  const onWheel = (e: WheelEvent) => {
    e.preventDefault()
    const r = svg.getBoundingClientRect()
    const factor = e.deltaY < 0 ? WHEEL_ZOOM_FACTOR : 1 / WHEEL_ZOOM_FACTOR
    setViewport(zoomAround(viewport(), { x: e.clientX - r.left, y: e.clientY - r.top }, factor))
  }

  onMount(() => {
    const ro = new ResizeObserver(measure)
    ro.observe(wrap)
    if (legendBox) ro.observe(legendBox)
    measure()
    svg.addEventListener('wheel', onWheel, { passive: false })
    svg.addEventListener('click', swallowClickAfterDrag, true)
    const unregister = registerCanvas({ svg, get: viewport, set: setViewport, fit })
    onCleanup(() => {
      unregister()
      ro.disconnect()
      svg.removeEventListener('wheel', onWheel)
      svg.removeEventListener('click', swallowClickAfterDrag, true)
    })
  })

  const api: CanvasApi = { viewport, setViewport, fit, fitTo, rect }
  const transform = () => {
    const v = viewport()
    return `translate(${v.x} ${v.y}) scale(${v.zoom})`
  }
  const dataBounds = () => `${props.bounds.x} ${props.bounds.y} ${props.bounds.w} ${props.bounds.h}`

  return (
    <CanvasContext.Provider value={api}>
      <div class="cv-wrap" ref={wrap}>
        <svg
          ref={svg}
          class="cv"
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={endDrag}
          onPointerLeave={endDrag}
        >
          <CanvasDefs />
          <g data-bounds={dataBounds()} transform={transform()}>
            {props.children}
          </g>
        </svg>
        <Show when={props.legend}>
          <div class="cv-legend" ref={legendBox}>
            {props.legend}
          </div>
        </Show>
        {props.overlay}
        <div class="zoom">
          <button onClick={() => zoomAtCenter(1 / BUTTON_ZOOM_FACTOR)} title={t('shell.canvas.zoomOut')}>
            −
          </button>
          <button onClick={() => zoomAtCenter(BUTTON_ZOOM_FACTOR)} title={t('shell.canvas.zoomIn')}>
            +
          </button>
          <button onClick={fit} title={t('shell.canvas.fitTitle')}>
            {t('shell.canvas.fit')}
          </button>
          <span class="zv">{Math.round(viewport().zoom * 100)}%</span>
        </div>
      </div>
    </CanvasContext.Provider>
  )
}

const CanvasDefs: Component = () => (
  <defs>
    <marker
      id={CANVAS_DEFS.arrowId}
      viewBox="0 0 8 8"
      refX="7"
      refY="4"
      markerUnits="userSpaceOnUse"
      markerWidth="7"
      markerHeight="7"
      orient="auto-start-reverse"
    >
      <path d="M0,0 L8,4 L0,8 z" fill="context-stroke" />
    </marker>
    <pattern id={CANVAS_DEFS.hatchId} width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
      <line x1="0" y1="0" x2="0" y2="6" stroke={HATCH_STROKE} stroke-width="1.2" />
    </pattern>
  </defs>
)

const svgEl = (name: string, attrs: Record<string, string | number>): SVGElement => {
  const el = document.createElementNS(SVG_NS, name)
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v))
  return el
}

export const exportSvg = (name: string): void => {
  const canvas = activeCanvas()
  if (!canvas) return
  const clone = canvas.svg.cloneNode(true) as SVGSVGElement
  const group = clone.querySelector('g[data-bounds]')
  if (!group) return
  const [x, y, w, h] = (group.getAttribute('data-bounds') ?? '').split(' ').map(Number)
  const pad = EXPORT_PADDING
  const box = { x: x - pad, y: y - pad, width: w + pad * 2, height: h + pad * 2 }
  group.removeAttribute('transform')
  clone.setAttribute('viewBox', `${box.x} ${box.y} ${box.width} ${box.height}`)
  clone.setAttribute('width', String(box.width))
  clone.setAttribute('height', String(box.height))
  clone.setAttribute('xmlns', SVG_NS)
  clone.removeAttribute('class')
  const background = svgEl('rect', { ...box, fill: EXPORT_BACKGROUND })
  const style = svgEl('style', {})
  style.textContent = "text{font-family:'JetBrains Mono',ui-monospace,monospace}"
  clone.insertBefore(background, group)
  clone.insertBefore(style, background)
  const blob = new Blob([new XMLSerializer().serializeToString(clone)], { type: 'image/svg+xml' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `${name}.svg`
  link.click()
}
