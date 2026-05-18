import { createResource, createSignal, Show } from 'solid-js'
import { loadSnapshot, SnapshotVersionMismatchError } from './lib/snapshot'
import { GraphSnapshot } from './types'

const App = () => {
  const [snapshot] = createResource<GraphSnapshot>(() => loadSnapshot('/api/snapshot'))
  const [showDebug] = createSignal(new URLSearchParams(location.search).has('debug'))

  return (
    <div style={{ 'font-family': 'system-ui, sans-serif', padding: '16px' }}>
      <h1>Stratum Visualizer</h1>
      <Show
        when={!snapshot.error}
        fallback={
          <ErrorView err={snapshot.error as Error} />
        }
      >
        <Show when={snapshot()} fallback={<p>Loading snapshot…</p>}>
          {(data) => (
            <div>
              <p>
                Modules: <strong>{data().modules.length}</strong> · Containers:{' '}
                <strong>{data().containers.length}</strong> · Layers:{' '}
                <strong>{data().layers.length}</strong> · Edges:{' '}
                <strong>{data().edges.length}</strong>
              </p>
              <Show when={showDebug()}>
                <pre style={{ background: '#f4f4f4', padding: '12px' }}>
                  {JSON.stringify(data(), null, 2)}
                </pre>
              </Show>
              <p>
                <em>
                  The PixiJS + dagre-wasm rendering layer is the Phase 8 follow-up; this MVP
                  build only proves the snapshot transport. See <code>_hot.md</code>.
                </em>
              </p>
            </div>
          )}
        </Show>
      </Show>
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
