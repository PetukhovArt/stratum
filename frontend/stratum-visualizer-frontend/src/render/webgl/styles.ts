// oklch → linear sRGB → 24-bit hex for Pixi tint/fill.
//
// Inputs are the same as CSS oklch():
//   l: 0..100 (percent)
//   c: 0..0.4 typical
//   h: 0..360 degrees
//
// Round-trip target is "close enough for prototype" — exact CSS color match
// is not required because the renderer is opt-in via a feature flag.

const oklabToLinear = (l: number, a: number, b: number): [number, number, number] => {
  const l_ = l + 0.3963377774 * a + 0.2158037573 * b
  const m_ = l - 0.1055613458 * a - 0.0638541728 * b
  const s_ = l - 0.0894841775 * a - 1.291485548 * b

  const lc = l_ * l_ * l_
  const mc = m_ * m_ * m_
  const sc = s_ * s_ * s_

  return [
    +4.0767416621 * lc - 3.3077115913 * mc + 0.2309699292 * sc,
    -1.2684380046 * lc + 2.6097574011 * mc - 0.3413193965 * sc,
    -0.0041960863 * lc - 0.7034186147 * mc + 1.707614701 * sc,
  ]
}

const linearToSrgb = (v: number): number => {
  if (v <= 0) return 0
  if (v >= 1) return 1
  return v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(v, 1 / 2.4) - 0.055
}

export const oklchToRgbHex = (l: number, c: number, h: number): number => {
  const hRad = (h * Math.PI) / 180
  const a = c * Math.cos(hRad)
  const b = c * Math.sin(hRad)
  const [rL, gL, bL] = oklabToLinear(l / 100, a, b)
  const r = Math.round(linearToSrgb(rL) * 255)
  const g = Math.round(linearToSrgb(gL) * 255)
  const bv = Math.round(linearToSrgb(bL) * 255)
  return (r << 16) | (g << 8) | bv
}

export const layerHueToHex = (hue: number, l = 60, c = 0.1): number =>
  oklchToRgbHex(l, c, hue)

const SEVERITY_FILL = {
  error: oklchToRgbHex(64, 0.18, 25),
  warning: oklchToRgbHex(78, 0.14, 70),
  info: oklchToRgbHex(70, 0.1, 230),
} as const

export type Severity = keyof typeof SEVERITY_FILL

export const severityFill = (s: Severity): number => SEVERITY_FILL[s]

export const severityGlow = severityFill

export const MODULE_FILL = oklchToRgbHex(18, 0.02, 250)
export const MODULE_STROKE = oklchToRgbHex(70, 0.02, 250)
export const CONTAINER_FILL = oklchToRgbHex(13, 0.02, 250)
export const LANE_FILL = oklchToRgbHex(11, 0.01, 250)
export const TEXT_COLOR = oklchToRgbHex(88, 0.02, 250)
