import { createEffect, createResource, createSignal, onCleanup, Show } from 'solid-js'
import { loadSnapshot, SnapshotVersionMismatchError } from './lib/snapshot'
import { renderGraph, RenderHandle } from './render'
import { GraphSnapshot } from './types'

const App = () => {
  const [snapshot] = createResource<GraphSnapshot>(() => loadSnapshot('/api/snapshot'))
  const [expanded, setExpanded] = createSignal<ReadonlySet<number>>(new Set())
  const [renderError, setRenderError] = createSignal<string | null>(null)
  let hostRef: HTMLDivElement | undefined
  let handle: RenderHandle | null = null

  createEffect(() => {
    const data = snapshot()
    if (!data || !hostRef) return
    handle?.destroy()
    handle = null
    setRenderError(null)
    renderGraph(hostRef, data, {
      expanded: expanded(),
      onAggregateClick: (layer) => {
        const next = new Set(expanded())
        next.add(layer)
        setExpanded(next)
      },
    })
      .then((h) => {
        handle = h
      })
      .catch((e: unknown) => setRenderError(e instanceof Error ? e.message : String(e)))
  })

  onCleanup(() => handle?.destroy())

  return (
    <div style={{ height: '100vh', display: 'flex', 'flex-direction': 'column' }}>
      <header
        style={{
          padding: '8px 16px',
          'font-family': 'system-ui, sans-serif',
          'border-bottom': '1px solid #333',
          background: '#111',
          color: '#eee',
        }}
      >
        <strong>Stratum Visualizer</strong>
        <Show when={snapshot()}>
          {(data) => (
            <span style={{ 'margin-left': '16px', color: '#888' }}>
              {data().modules.length} modules · {data().containers.length} containers ·{' '}
              {data().layers.length} layers · {data().edges.length} edges
            </span>
          )}
        </Show>
      </header>
      <Show when={snapshot.error}>
        <ErrorView err={snapshot.error as Error} />
      </Show>
      <Show when={renderError()}>
        <pre style={{ color: '#f88', padding: '12px', margin: 0 }}>{renderError()}</pre>
      </Show>
      <div ref={hostRef} style={{ flex: '1', 'min-height': 0 }} />
    </div>
  )
}

const ErrorView = (props: { err: Error }) => {
  const isVersionMismatch = props.err instanceof SnapshotVersionMismatchError
  return (
    <div role="alert" style={{ color: '#a00', padding: '12px', border: '1px solid #a00' }}>
      <strong>{isVersionMismatch ? 'Visualizer out of date' : 'Snapshot error'}</strong>
      <p>{props.err.message}</p>
    </div>
  )
}

export default App
