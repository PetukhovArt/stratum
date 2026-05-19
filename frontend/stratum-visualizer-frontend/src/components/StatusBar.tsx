import { Component, createMemo, Show } from 'solid-js'
import { DesignData, DesignModule } from '../render/design'
import { Filters, Viewport } from '../state'

export const StatusBar: Component<{
  data: DesignData
  viewport: Viewport
  selected: DesignModule | null
  leftCollapsed: boolean
  setLeftCollapsed: (v: boolean) => void
  rightCollapsed: boolean
  setRightCollapsed: (v: boolean) => void
  filters: Filters
}> = (props) => {
  const activeFilters = createMemo(() =>
    props.filters.disabledLayers.size +
    props.filters.disabledContainers.size +
    (props.filters.onlyViolators ? 1 : 0) +
    (props.filters.query ? 1 : 0) +
    (props.filters.glob ? 1 : 0) +
    (props.filters.stageFilter !== null ? 1 : 0),
  )

  return (
    <footer class="strat-status">
      <div class="status-l">
        <button
          class={`status-btn ${props.leftCollapsed ? 'off' : ''}`}
          onClick={() => props.setLeftCollapsed(!props.leftCollapsed)}
          title="Toggle outline panel (⌘[)"
        >
          <PanelIcon side="left" collapsed={props.leftCollapsed} />
          <span class="status-btn-label">outline</span>
        </button>
        <span class="status-sep" />
        <span class="status-info">
          {props.data.modules.length} modules · {props.data.containers.length} containers ·{' '}
          {props.data.edges.length} edges
        </span>
      </div>
      <div class="status-c">
        <Show when={props.selected} fallback={<span class="status-dim">no selection</span>}>
          {(s) => <span class="status-mono">{s().path}</span>}
        </Show>
      </div>
      <div class="status-r">
        <Show when={activeFilters() > 0}>
          <span class="status-pill">
            ⚙ {activeFilters()} filter{activeFilters() > 1 ? 's' : ''} active
          </span>
        </Show>
        <span class="status-info">zoom {Math.round(props.viewport.zoom * 100)}%</span>
        <span class="status-sep" />
        <button
          class={`status-btn ${props.rightCollapsed ? 'off' : ''}`}
          onClick={() => props.setRightCollapsed(!props.rightCollapsed)}
          title="Toggle details panel (⌘])"
        >
          <span class="status-btn-label">details</span>
          <PanelIcon side="right" collapsed={props.rightCollapsed} />
        </button>
      </div>
    </footer>
  )
}

const PanelIcon: Component<{ side: 'left' | 'right'; collapsed: boolean }> = (props) => {
  const left = () => props.side === 'left'
  return (
    <svg width="14" height="11" viewBox="0 0 14 11" class="panel-icon">
      <rect
        x={0.5}
        y={0.5}
        width={13}
        height={10}
        rx={1.2}
        fill="none"
        stroke="currentColor"
        stroke-width={1}
      />
      <rect
        x={left() ? 0.5 : 9}
        y={0.5}
        width={4.5}
        height={10}
        fill={props.collapsed ? 'transparent' : 'currentColor'}
        opacity={props.collapsed ? 0 : 0.7}
      />
    </svg>
  )
}

export const ZoomControls: Component<{
  viewport: Viewport
  setViewport: (v: Viewport) => void
  fit: () => void
  stageSize: { w: number; h: number }
}> = (props) => {
  const zoomBy = (factor: number) => {
    const mx = props.stageSize.w / 2
    const my = props.stageSize.h / 2
    const v = props.viewport
    const newZoom = Math.max(0.15, Math.min(4, v.zoom * factor))
    const wx = (mx - v.x) / v.zoom
    const wy = (my - v.y) / v.zoom
    props.setViewport({ zoom: newZoom, x: mx - wx * newZoom, y: my - wy * newZoom })
  }
  return (
    <div class="zoom-ctrl">
      <button onClick={() => zoomBy(1.2)} title="zoom in">
        +
      </button>
      <button onClick={() => zoomBy(1 / 1.2)} title="zoom out">
        −
      </button>
      <button onClick={() => props.fit()} title="fit">
        ⤢
      </button>
      <div class="zoom-val">{Math.round(props.viewport.zoom * 100)}%</div>
    </div>
  )
}

export const Legend: Component = () => (
  <div class="legend">
    <div class="legend-row">
      <span class="legend-swatch sw-vio" />
      <span>violation</span>
    </div>
    <div class="legend-row">
      <span class="legend-glyph">◇</span>
      <span>composer</span>
    </div>
    <div class="legend-row">
      <span class="legend-glyph legend-glyph-s">_</span>
      <span>_shared (segment-private)</span>
    </div>
    <div class="legend-row">
      <span class="legend-thick" />
      <span>fractal (has children)</span>
    </div>
  </div>
)
