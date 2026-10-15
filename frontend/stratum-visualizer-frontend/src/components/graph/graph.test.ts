import { createComponent, createSignal } from 'solid-js'
import { render } from 'solid-js/web'
import { describe, expect, it } from 'vitest'
import { adaptSnapshot } from '../../render/adapt'
import { computeLayout } from '../../render/layout'
import { DEFAULT_FILTERS, Viewport } from '../../state'
import { EdgeKind, GraphSnapshot } from '../../types'
import { Graph } from './graph'

const snap: GraphSnapshot = {
  version: 1,
  modules: [
    { id: 1, path: 'src/a.ts', container: 10, layer: 0, stage: 1 },
    { id: 2, path: 'src/b.ts', container: 10, layer: 0, stage: 1 },
    { id: 3, path: 'src/c.ts', container: 11, layer: 1, stage: 1 },
  ],
  containers: [
    { id: 10, name: 'features', layer: 0, parent: null },
    { id: 11, name: 'shared', layer: 1, parent: null },
  ],
  layers: [
    { id: 0, name: 'features', depends_on: [1] },
    { id: 1, name: 'shared', depends_on: [] },
  ],
  edges: [{ from: 1, to: 3, kind: EdgeKind.Static }],
}

describe('Graph', () => {
  it('keeps every rendered module node when the viewport pans', () => {
    const data = adaptSnapshot(snap)
    const scene = computeLayout(data)
    const [viewport, setViewport] = createSignal<Viewport>({ x: 0, y: 0, zoom: 1 })
    const host = document.createElement('div')
    const dispose = render(
      () =>
        createComponent(Graph, {
          scene,
          data,
          filters: DEFAULT_FILTERS(),
          isHaloThick: true,
          selectedId: null,
          hoveredId: null,
          focusedCycle: null,
          onSelect: () => {},
          onHover: () => {},
          onHoverEdge: () => {},
          hoveredEdge: null,
          get viewport() {
            return viewport()
          },
          canvasSize: { w: 2000, h: 2000 },
          onViewportChange: setViewport,
        }),
      host,
    )
    const before = [...host.querySelectorAll('g[data-mod-id]')]

    setViewport({ x: 5, y: 7, zoom: 1 })

    const after = [...host.querySelectorAll('g[data-mod-id]')]
    expect(before.length).toBeGreaterThan(0)
    expect(after).toHaveLength(before.length)
    after.forEach((node, i) => expect(node).toBe(before[i]))
    dispose()
  })
})
