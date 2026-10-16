import { CanvasSize, Viewport } from '../state'

export const MIN_ZOOM = 0.15
export const MAX_ZOOM = 4
const MAX_FIT_ZOOM = 2
const FIT_PADDING_PX = 40
const CENTER_ON_MIN_ZOOM = 0.85

export const DEFAULT_VIEWPORT: Viewport = { x: 0, y: 0, zoom: 0.5 }

const clampZoom = (zoom: number, max = MAX_ZOOM): number => Math.max(MIN_ZOOM, Math.min(max, zoom))

export const zoomAround = (v: Viewport, point: { x: number; y: number }, factor: number): Viewport => {
  const zoom = clampZoom(v.zoom * factor)
  const worldX = (point.x - v.x) / v.zoom
  const worldY = (point.y - v.y) / v.zoom
  return { zoom, x: point.x - worldX * zoom, y: point.y - worldY * zoom }
}

export interface Bounds {
  x: number
  y: number
  w: number
  h: number
}

export const fitViewport = (bounds: Bounds, size: CanvasSize): Viewport => {
  if (!size.w || !bounds.w || !bounds.h) return DEFAULT_VIEWPORT
  const fx = (size.w - FIT_PADDING_PX * 2) / bounds.w
  const fy = (size.h - FIT_PADDING_PX * 2) / bounds.h
  const zoom = clampZoom(Math.min(fx, fy), MAX_FIT_ZOOM)
  return {
    zoom,
    x: (size.w - bounds.w * zoom) / 2 - bounds.x * zoom,
    y: (size.h - bounds.h * zoom) / 2 - bounds.y * zoom,
  }
}

export const centerOn = (
  v: Viewport,
  point: { x: number; y: number },
  size: CanvasSize,
): Viewport => {
  const zoom = Math.max(v.zoom, CENTER_ON_MIN_ZOOM)
  return { zoom, x: size.w / 2 - point.x * zoom, y: size.h / 2 - point.y * zoom }
}
