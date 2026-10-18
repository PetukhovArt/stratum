import { Viewport } from '../state'

export interface ActiveCanvas {
  svg: SVGSVGElement
  get: () => Viewport
  set: (v: Viewport) => void
  fit: () => void
}

let active: ActiveCanvas | null = null

export const registerCanvas = (canvas: ActiveCanvas): (() => void) => {
  active = canvas
  return () => {
    if (active === canvas) active = null
  }
}

export const activeCanvas = (): ActiveCanvas | null => active
