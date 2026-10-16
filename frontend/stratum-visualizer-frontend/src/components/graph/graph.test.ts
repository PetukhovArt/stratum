import { createComponent, createSignal } from 'solid-js'
import { render } from 'solid-js/web'
import { describe, expect, it } from 'vitest'
import { designOf, SnapshotInput } from '../../test/snapshot'
import { computeLayout } from '../../render/layout'
import { DEFAULT_FILTERS, Viewport } from '../../state'
import { EdgeKind } from '../../types'
import { Graph } from './graph'

const snap: SnapshotInput = {
  modules: [
    { id: 1, path: 'src/a.ts', container: 10, layer: 0, purity: 1 },
    { id: 2, path: 'src/b.ts', container: 10, layer: 0, purity: 1 },
    { id: 3, path: 'src/c.ts', container: 11, layer: 1, purity: 1 },
  ],
  containers: [
    { id: 10, name: 'features', path: 'src/features', kind: 'layer', layer: 0, parent: null, stage: 2, entry: null },
    { id: 11, name: 'shared', path: 'src/shared', kind: 'layer', layer: 1, parent: null, stage: 2, entry: null },
  ],
  layers: [
    { id: 0, name: 'features', depends_on: [1] },
    { id: 1, name: 'shared', depends_on: [] },
  ],
  edges: [{ from: 1, to: 3, kind: EdgeKind.Static }],
}

describe('Graph', () => {
  it('keeps every rendered module node when the viewport pans', () => {
    const data = designOf(snap)
    const scene = computeLayout(data)
    const [viewport, setViewport] = createSignal<Viewport>({ x: 0, y: 0, zoom: 1 })
    const host = document.createElement('div')
    const dispose = render(
      () =>
        createComponent(Graph, {
          scene,
          data,
          filters: DEFAULT_FILTERS(),
          colorMode: 'layer',
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
