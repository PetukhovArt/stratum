import { vi } from 'vitest'

class FakeCanvasCtx {
  font = ''
  textBaseline = 'alphabetic'
  fillStyle = '#000'
  strokeStyle = '#000'
  globalAlpha = 1
  lineWidth = 1
  fillRect = vi.fn()
  clearRect = vi.fn()
  beginPath = vi.fn()
  moveTo = vi.fn()
  lineTo = vi.fn()
  stroke = vi.fn()
  fill = vi.fn()
  arc = vi.fn()
  closePath = vi.fn()
  save = vi.fn()
  restore = vi.fn()
  translate = vi.fn()
  scale = vi.fn()
  rotate = vi.fn()
  rect = vi.fn()
  measureText = vi.fn(() => ({ width: 0 }))
}

HTMLCanvasElement.prototype.getContext = vi.fn(
  () => new FakeCanvasCtx(),
) as unknown as typeof HTMLCanvasElement.prototype.getContext

const globalAny = globalThis as Record<string, unknown>
if (!globalAny.CanvasRenderingContext2D) {
  globalAny.CanvasRenderingContext2D = FakeCanvasCtx
}

