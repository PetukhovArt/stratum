import { Accessor, Component, createSignal, onCleanup, onMount, Setter, Show } from 'solid-js'
import { CanvasApi, Canvas, useCanvas } from '../../canvas/canvas'
import { DetailsPanel } from '../../components/details'
import { FacetBar } from '../../components/facet-bar'
import { Graph } from '../../components/graph/graph'
import { HoverTooltip } from '../../components/hover-tooltip'
import { Legend } from '../../components/legend'
import { Minimap } from '../../components/minimap'
import { OutlinePanel } from '../../components/outline/outline-panel'
import { adaptModel, designKey, parseDesignKey } from '../../render/adapt'
import { DesignData } from '../../render/design'
import { computeLayout, Scene } from '../../render/layout'
import { Model } from '../../render/model'
import { centerOn } from '../../render/viewport'
import { nav, set } from '../../shell/nav'
import { ColorMode, DEFAULT_FILTERS, Filters, HoveredEdge, HoveredMod } from '../../state'
import { ViewDef, ViewProps } from '../view-def'

interface OverviewState {
  data: DesignData
  scene: Scene
  filters: Accessor<Filters>
  setFilters: Setter<Filters>
  colorMode: Accessor<ColorMode>
  setColorMode: Setter<ColorMode>
  isHaloThick: Accessor<boolean>
  setHaloThick: Setter<boolean>
  hovered: Accessor<HoveredMod | null>
  setHovered: Setter<HoveredMod | null>
  hoveredEdge: Accessor<HoveredEdge | null>
  setHoveredEdge: Setter<HoveredEdge | null>
  focusedCycle: Accessor<string | null>
  setFocusedCycle: Setter<string | null>
  selectedId: () => string | null
  select: (id: string | null) => void
  attachCanvas: (api: CanvasApi) => () => void
}

const createOverviewState = (model: Model): OverviewState => {
  const data = adaptModel(model)
  const scene = computeLayout(data)
  const [filters, setFilters] = createSignal<Filters>(DEFAULT_FILTERS())
  const [colorMode, setColorMode] = createSignal<ColorMode>('layer')
  const [isHaloThick, setHaloThick] = createSignal(true)
  const [hovered, setHovered] = createSignal<HoveredMod | null>(null)
  const [hoveredEdge, setHoveredEdge] = createSignal<HoveredEdge | null>(null)
  const [focusedCycle, setFocusedCycle] = createSignal<string | null>(null)
  let canvas: CanvasApi | null = null

  const selectedId = () => {
    const sel = nav().sel
    return sel && (sel.kind === 'module' || sel.kind === 'container') ? designKey(sel) : null
  }

  const centerOnModule = (id: string) => {
    const pos = scene.modulePos[id]
    const rect = canvas?.rect()
    if (!canvas || !pos || !rect) return
    canvas.setViewport(
      centerOn(canvas.viewport(), { x: pos.cx, y: pos.cy }, { w: rect.width, h: rect.height }),
    )
  }

  const select = (id: string | null) => {
    set({ sel: id ? parseDesignKey(id) : null })
    if (id) centerOnModule(id)
  }

  const attachCanvas = (api: CanvasApi) => {
    canvas = api
    return () => {
      if (canvas === api) canvas = null
    }
  }

  return {
    data,
    scene,
    filters,
    setFilters,
    colorMode,
    setColorMode,
    isHaloThick,
    setHaloThick,
    hovered,
    setHovered,
    hoveredEdge,
    setHoveredEdge,
    focusedCycle,
    setFocusedCycle,
    selectedId,
    select,
    attachCanvas,
  }
}

const states = new WeakMap<Model, OverviewState>()

const overviewState = (model: Model): OverviewState => {
  const cached = states.get(model)
  if (cached) return cached
  const created = createOverviewState(model)
  states.set(model, created)
  return created
}

const OverviewGraph: Component<{ ov: OverviewState }> = (props) => {
  const canvas = useCanvas()
  onMount(() => onCleanup(props.ov.attachCanvas(canvas)))
  const size = () => {
    const r = canvas.rect()
    return r ? { w: r.width, h: r.height } : { w: 0, h: 0 }
  }
  return (
    <Graph
      scene={props.ov.scene}
      data={props.ov.data}
      filters={props.ov.filters()}
      colorMode={props.ov.colorMode()}
      isHaloThick={props.ov.isHaloThick()}
      selectedId={props.ov.selectedId()}
      hoveredId={props.ov.hovered()?.id ?? null}
      focusedCycle={props.ov.focusedCycle()}
      onSelect={props.ov.select}
      onHover={props.ov.setHovered}
      onHoverEdge={props.ov.setHoveredEdge}
      hoveredEdge={props.ov.hoveredEdge()}
      viewport={canvas.viewport()}
      canvasSize={size()}
    />
  )
}

const OverviewOverlay: Component<{ ov: OverviewState }> = (props) => {
  const canvas = useCanvas()
  const size = () => {
    const r = canvas.rect()
    return r ? { w: r.width, h: r.height } : { w: 0, h: 0 }
  }
  return (
    <>
      <Legend />
      <Show when={props.ov.focusedCycle()}>
        {(fc) => (
          <div class="cycle-banner">
            <span class="cycle-dot" />
            <span>Focused on cycle</span>
            <code>{props.ov.data.violationById.get(fc())?.message ?? ''}</code>
            <button onClick={() => props.ov.setFocusedCycle(null)}>exit ↩</button>
          </div>
        )}
      </Show>
      <Minimap
        scene={props.ov.scene}
        viewport={canvas.viewport()}
        onViewportChange={canvas.setViewport}
        canvasSize={size()}
      />
      <HoverTooltip
        data={props.ov.data}
        hoveredModule={props.ov.hovered()}
        hoveredEdge={props.ov.hoveredEdge()}
        canvasRect={canvas.rect()}
      />
    </>
  )
}

const Stage: Component<ViewProps> = (props) => {
  const ov = overviewState(props.model)
  const bounds = { x: 0, y: 0, w: ov.scene.width, h: ov.scene.height }
  return (
    <div class="ov-stage">
      <FacetBar
        data={ov.data}
        filters={ov.filters()}
        setFilters={ov.setFilters}
        colorMode={ov.colorMode()}
        setColorMode={ov.setColorMode}
        isHaloThick={ov.isHaloThick()}
        onHaloToggle={() => ov.setHaloThick(!ov.isHaloThick())}
      />
      <div class="ov-canvas">
        <Canvas bounds={bounds} fitKey={ov.scene} overlay={<OverviewOverlay ov={ov} />}>
          <OverviewGraph ov={ov} />
        </Canvas>
      </div>
    </div>
  )
}

const Outline: Component<ViewProps> = (props) => {
  const ov = overviewState(props.model)
  return (
    <OutlinePanel
      data={ov.data}
      filters={ov.filters()}
      setFilters={ov.setFilters}
      selectedId={ov.selectedId()}
      onSelect={ov.select}
    />
  )
}

const Details: Component<ViewProps> = (props) => {
  const ov = overviewState(props.model)
  return (
    <DetailsPanel
      data={ov.data}
      selectedId={ov.selectedId()}
      hoveredId={ov.hovered()?.id ?? null}
      onClose={() => ov.select(null)}
      onFocusCycle={(vid) => ov.setFocusedCycle(ov.focusedCycle() === vid ? null : vid)}
      focusedCycle={ov.focusedCycle()}
      onSelect={ov.select}
      isViolationsUnavailable={props.isViolationsUnavailable}
    />
  )
}

export const overviewView: ViewDef = {
  id: 'overview',
  key: '4',
  title: 'view.overview',
  Stage,
  Outline,
  Details,
}
