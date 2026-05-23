import { describe, expect, it } from 'vitest'
import { oklchToRgbHex, severityFill, layerHueToHex } from './styles'

describe('oklchToRgbHex', () => {
  it('converts pure white (oklch(100% 0 0)) to 0xffffff', () => {
    expect(oklchToRgbHex(100, 0, 0)).toBe(0xffffff)
  })

  it('converts pure black (oklch(0% 0 0)) to 0x000000', () => {
    expect(oklchToRgbHex(0, 0, 0)).toBe(0x000000)
  })

  it('produces a 24-bit integer for arbitrary oklch input', () => {
    const v = oklchToRgbHex(64, 0.18, 25)
    expect(v).toBeGreaterThanOrEqual(0)
    expect(v).toBeLessThanOrEqual(0xffffff)
    expect(Number.isInteger(v)).toBe(true)
  })

  it('is deterministic — same input gives same output', () => {
    expect(oklchToRgbHex(64, 0.18, 25)).toBe(oklchToRgbHex(64, 0.18, 25))
  })
})

describe('severityFill', () => {
  it('returns distinct hex values for error / warning / info', () => {
    const err = severityFill('error')
    const warn = severityFill('warning')
    const info = severityFill('info')
    expect(err).not.toBe(warn)
    expect(warn).not.toBe(info)
    expect(err).not.toBe(info)
  })
})

describe('layerHueToHex', () => {
  it('maps hue to a deterministic hex value', () => {
    expect(layerHueToHex(120)).toBe(layerHueToHex(120))
  })
})
