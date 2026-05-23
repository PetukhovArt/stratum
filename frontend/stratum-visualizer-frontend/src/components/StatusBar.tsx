import { Component, createMemo, createSignal, Show } from 'solid-js'
import { DesignData, DesignModule } from '../render/design'
import { Filters, Viewport } from '../state'
import { Tooltip } from './Tooltip'

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
        <Tooltip content="Toggle outline panel (⌘[)">
          <button
            class={`status-btn ${props.leftCollapsed ? 'off' : ''}`}
            onClick={() => props.setLeftCollapsed(!props.leftCollapsed)}
          >
            <PanelIcon side="left" collapsed={props.leftCollapsed} />
            <span class="status-btn-label">outline</span>
          </button>
        </Tooltip>
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
        <Tooltip content="Toggle details panel (⌘])">
          <button
            class={`status-btn ${props.rightCollapsed ? 'off' : ''}`}
            onClick={() => props.setRightCollapsed(!props.rightCollapsed)}
          >
            <span class="status-btn-label">details</span>
            <PanelIcon side="right" collapsed={props.rightCollapsed} />
          </button>
        </Tooltip>
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
      <Tooltip content="Zoom in">
        <button onClick={() => zoomBy(1.2)}>+</button>
      </Tooltip>
      <Tooltip content="Zoom out">
        <button onClick={() => zoomBy(1 / 1.2)}>−</button>
      </Tooltip>
      <Tooltip content="Fit to bounds (F or Esc)">
        <button onClick={() => props.fit()}>⤢</button>
      </Tooltip>
      <div class="zoom-val">{Math.round(props.viewport.zoom * 100)}%</div>
    </div>
  )
}

export const Legend: Component = () => {
  const [open, setOpen] = createSignal(true)
  return (
    <div class="legend">
      <div class="legend-hd" onClick={() => setOpen(!open())}>
        <span class="legend-hd-label">Legend</span>
        <span class={`legend-hd-chev ${open() ? 'open' : ''}`}>▸</span>
      </div>
      <Show when={open()}>
        <div class="legend-section-label">Lane</div>
        <div class="legend-row">
          <span class="legend-lane" />
          <span>layer swimlane</span>
        </div>

        <div class="legend-sep" />
        <div class="legend-section-label">Module body</div>
        <div class="legend-row">
          <span class="legend-mod legend-mod-regular" />
          <span>regular leaf · stage colored</span>
        </div>
        <div class="legend-row">
          <span class="legend-mod legend-mod-composer" />
          <span>◇ composer (orchestrator)</span>
        </div>
        <div class="legend-row">
          <span class="legend-mod legend-mod-shared" />
          <span>_shared (segment-private)</span>
        </div>
        <div class="legend-row">
          <span class="legend-mod legend-mod-compound" />
          <span>compound · contains children</span>
        </div>
        <div class="legend-row">
          <span class="legend-mod legend-mod-selected" />
          <span>selected · focus ring</span>
        </div>

        <div class="legend-sep" />
        <div class="legend-section-label">Edges</div>
        <div class="legend-row">
          <span class="legend-swatch" />
          <span>static import</span>
        </div>
        <div class="legend-row">
          <span class="legend-swatch sw-di" />
          <span>DI container</span>
        </div>
        <div class="legend-row">
          <span class="legend-swatch sw-runtime" />
          <span>runtime / dynamic</span>
        </div>
        <div class="legend-row">
          <span class="legend-swatch sw-vio" />
          <span>violation (pulses)</span>
        </div>
        <div class="legend-row">
          <span class="legend-swatch sw-out" />
          <span>outgoing (on hover)</span>
        </div>
        <div class="legend-row">
          <span class="legend-swatch sw-in" />
          <span>incoming (on hover)</span>
        </div>

        <div class="legend-sep" />
        <div class="legend-section-label">Stages</div>
        <div class="legend-row">
          <span class="legend-stage" data-stage="1">S1</span>
          <span>file</span>
        </div>
        <div class="legend-row">
          <span class="legend-stage" data-stage="2">S2</span>
          <span>folder</span>
        </div>
        <div class="legend-row">
          <span class="legend-stage" data-stage="3">S3</span>
          <span>segments</span>
        </div>
        <div class="legend-row">
          <span class="legend-stage" data-stage="4">S4</span>
          <span>composed (fractal)</span>
        </div>

        <div class="legend-sep" />
        <div class="legend-section-label">Severity</div>
        <div class="legend-row">
          <span class="legend-sev sev-err" />
          <span>error</span>
        </div>
        <div class="legend-row">
          <span class="legend-sev sev-warn" />
          <span>warning</span>
        </div>
        <div class="legend-row">
          <span class="legend-sev sev-info" />
          <span>info</span>
        </div>
        <div class="legend-row">
          <span class="legend-badge">
            <span style={{ color: 'oklch(64% 0.18 25)' }}>●</span>
            <span style={{ color: 'oklch(78% 0.14 70)' }}>▲</span>
          </span>
          <span>aggregate counts</span>
        </div>
      </Show>
    </div>
  )
}
