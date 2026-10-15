import { Component, Show } from 'solid-js'
import { activeFilterCount } from '../lib/filters'
import { DesignData, DesignModule } from '../render/design'
import { zoomAround } from '../render/viewport'
import { CanvasSize, Filters, Viewport } from '../state'

const BUTTON_ZOOM_FACTOR = 1.2

export const StatusBar: Component<{
  data: DesignData
  viewport: Viewport
  selected: DesignModule | null
  isLeftCollapsed: boolean
  setLeftCollapsed: (v: boolean) => void
  isRightCollapsed: boolean
  setRightCollapsed: (v: boolean) => void
  filters: Filters
}> = (props) => {
  const activeFilters = () => activeFilterCount(props.filters)

  return (
    <footer class="strat-status">
      <div class="status-l">
        <button
          class={`status-btn ${props.isLeftCollapsed ? 'off' : ''}`}
          onClick={() => props.setLeftCollapsed(!props.isLeftCollapsed)}
          title="Toggle outline panel (⌘[)"
        >
          <PanelIcon side="left" isCollapsed={props.isLeftCollapsed} />
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
          class={`status-btn ${props.isRightCollapsed ? 'off' : ''}`}
          onClick={() => props.setRightCollapsed(!props.isRightCollapsed)}
          title="Toggle details panel (⌘])"
        >
          <span class="status-btn-label">details</span>
          <PanelIcon side="right" isCollapsed={props.isRightCollapsed} />
        </button>
      </div>
    </footer>
  )
}

const PanelIcon: Component<{ side: 'left' | 'right'; isCollapsed: boolean }> = (props) => {
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
        fill={props.isCollapsed ? 'transparent' : 'currentColor'}
        opacity={props.isCollapsed ? 0 : 0.7}
      />
    </svg>
  )
}

export const ZoomControls: Component<{
  viewport: Viewport
  setViewport: (v: Viewport) => void
  fit: () => void
  canvasSize: CanvasSize
}> = (props) => {
  const zoomBy = (factor: number) =>
    props.setViewport(
      zoomAround(props.viewport, { x: props.canvasSize.w / 2, y: props.canvasSize.h / 2 }, factor),
    )
  return (
    <div class="zoom-ctrl">
      <button onClick={() => zoomBy(BUTTON_ZOOM_FACTOR)} title="zoom in">
        +
      </button>
      <button onClick={() => zoomBy(1 / BUTTON_ZOOM_FACTOR)} title="zoom out">
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
