import { createSignal } from 'solid-js'

const STORAGE_KEY = 'stratum.legend'

const [legendOpen, setLegendOpenSignal] = createSignal(localStorage.getItem(STORAGE_KEY) !== 'closed')

export { legendOpen }

export const toggleLegend = (): void => {
  const next = !legendOpen()
  setLegendOpenSignal(next)
  localStorage.setItem(STORAGE_KEY, next ? 'open' : 'closed')
}
