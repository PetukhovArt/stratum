import { Component, createMemo, For, Show } from 'solid-js'
import { moduleMatcher } from '../../lib/filters'
import { ancestorIds, DesignData } from '../../render/design'
import { LaidEdge, Scene } from '../../render/layout'
import { CanvasSize, ColorMode, Filters, HoveredEdge, HoveredMod, Viewport } from '../../state'
import { ContainerBox } from './container-box'
import { EdgeDirection, EdgeLine, EdgeMarkers } from './edge-line'
import { Lane } from './lane'
import { ModuleNode } from './module-node'

const CULL_MARGIN_PX = 200
const MIN_RENDERED_MODULE_PX = 12
const VIOLATIONS_ONLY_BELOW_ZOOM = 0.3
const MAX_INTERACTIVE_EDGES = 500

const buildVisibility = (data: DesignData, filters: Filters): Set<string> => {
  const matches = moduleMatcher(filters, data)
  const visible = new Set<string>()
  for (const mod of data.modules) {
    const layer = data.layerByContainer.get(mod.container)?.id ?? ''
    const isHiddenByEye =
      filters.disabledLayers.has(layer) || filters.disabledContainers.has(mod.container)
    if (isHiddenByEye || !matches(mod)) continue
    visible.add(mod.id)
    for (const a of ancestorIds(data.moduleById, mod.id)) visible.add(a)
  }
  return visible
}

interface GraphProps {
  scene: Scene
  data: DesignData
  filters: Filters
  colorMode: ColorMode
  isHaloThick: boolean
  selectedId: string | null
  hoveredId: string | null
  focusedCycle: string | null
  onSelect: (id: string) => void
  onHover: (h: HoveredMod | null) => void
  onHoverEdge: (h: HoveredEdge | null) => void
  hoveredEdge: HoveredEdge | null
  viewport: Viewport
  canvasSize: CanvasSize
}

export const Graph: Component<GraphProps> = (props) => {

  const visibleMods = createMemo(() => buildVisibility(props.data, props.filters))

  const hueByContainer = createMemo(() => {
    const m = new Map<string, number>()
    for (const [cid, layer] of props.data.layerByContainer) m.set(cid, layer.hue)
    return m
  })

  const edgesByMod = createMemo(() => {
    const m = new Map<string, LaidEdge[]>()
    const add = (id: string, e: LaidEdge): void => {
      const list = m.get(id) ?? []
      list.push(e)
      m.set(id, list)
    }
    for (const e of props.scene.edges) {
      add(e.source, e)
      add(e.target, e)
    }
    return m
  })

  const cycleModIds = createMemo(() => {
    if (!props.focusedCycle) return null
    return new Set(props.data.violationById.get(props.focusedCycle)?.modules ?? [])
  })

  const isCycleEdge = (e: LaidEdge): boolean => {
    const cy = cycleModIds()
    return cy !== null && cy.has(e.source) && cy.has(e.target)
  }

  const highlight = () => props.hoveredId ?? props.selectedId

  const neighbours = createMemo(() => {
    const h = highlight()
    if (!h) return null
    const set = new Set<string>([h])
    for (const e of edgesByMod().get(h) ?? []) {
      set.add(e.source === h ? e.target : e.source)
    }
    for (const a of ancestorIds(props.data.moduleById, h)) set.add(a)
    return set
  })

  const isModuleDimmed = (modId: string): boolean => {
    const cy = cycleModIds()
    if (cy) return !cy.has(modId)
    if (!visibleMods().has(modId)) return true
    const nb = neighbours()
    return nb !== null && !nb.has(modId)
  }

  const edgeDirection = (e: LaidEdge): EdgeDirection => {
    const h = highlight()
    if (!h) return null
    if (e.source === h) return 'out'
    if (e.target === h) return 'in'
    return null
  }

  const isEdgeShown = (e: LaidEdge): boolean => {
    if (cycleModIds()) return isCycleEdge(e)
    if (props.filters.isViolatorsOnly && !e.violation) return false
    if (!visibleMods().has(e.source) || !visibleMods().has(e.target)) return false
    if (!props.filters.edgeKinds.has(e.kind)) return false
    const isMinimalHidden =
      props.filters.connectionMode === 'minimal' && !e.violation && edgeDirection(e) === null
    return !isMinimalHidden
  }

  const isEdgeDimmed = (e: LaidEdge): boolean =>
    !props.focusedCycle && highlight() !== null && edgeDirection(e) === null

  const viewBounds = createMemo(() => {
    const v = props.viewport
    return {
      x1: (-v.x - CULL_MARGIN_PX) / v.zoom,
      y1: (-v.y - CULL_MARGIN_PX) / v.zoom,
      x2: (props.canvasSize.w - v.x + CULL_MARGIN_PX) / v.zoom,
      y2: (props.canvasSize.h - v.y + CULL_MARGIN_PX) / v.zoom,
    }
  })

  const isInView = (x: number, y: number, w: number, h: number): boolean => {
    const b = viewBounds()
    return x < b.x2 && x + w > b.x1 && y < b.y2 && y + h > b.y1
  }

  const modulesByDepth = createMemo(() =>
    Object.entries(props.scene.modulePos).sort((a, b) => a[1].depth - b[1].depth),
  )

  const renderedModules = createMemo(() => {
    const zoom = props.viewport.zoom
    return modulesByDepth().filter(([, p]) => {
      if (!isInView(p.x, p.y, p.w, p.h)) return false
      const isTopCompound = p.depth === 0 && !p.isLeaf
      return isTopCompound || p.mod.severity !== null || p.w * zoom >= MIN_RENDERED_MODULE_PX
    })
  })

  const renderedContainers = createMemo(() =>
    props.scene.containers.filter((c) => isInView(c.absX, c.absY, c.width, c.height)),
  )

  const renderedEdges = createMemo(() => {
    const isViolationsOnly = props.viewport.zoom < VIOLATIONS_ONLY_BELOW_ZOOM
    return props.scene.edges.filter((e) => {
      if (isViolationsOnly && !e.violation) return false
      const x = Math.min(e.x1, e.x2)
      const y = Math.min(e.y1, e.y2)
      return isInView(x, y, Math.abs(e.x2 - e.x1), Math.abs(e.y2 - e.y1))
    })
  })

  const isEdgeInteractive = () => props.scene.edges.length <= MAX_INTERACTIVE_EDGES

  return (
    <g class="strat-graph">
      <defs>
        <EdgeMarkers />
        <pattern id="shared-hatch" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="oklch(72% 0.08 290 / .35)" stroke-width="1.2" />
        </pattern>
      </defs>

      <g>
        <For each={props.scene.lanes}>
          {(lane) => (
            <Lane
              lane={lane}
              isDimmed={props.filters.disabledLayers.has(lane.id)}
              sceneWidth={props.scene.width}
            />
          )}
        </For>

        <g class="containers">
          <For each={renderedContainers()}>
            {(c) => (
              <ContainerBox
                container={c}
                hue={hueByContainer().get(c.id)}
                isDimmed={
                  props.filters.disabledContainers.has(c.id) ||
                  props.filters.disabledLayers.has(c.layer)
                }
              />
            )}
          </For>
        </g>

        <g class="edges">
          <For each={renderedEdges()}>
            {(e) => (
              <Show when={isEdgeShown(e)}>
                <EdgeLine
                  edge={e}
                  direction={edgeDirection(e)}
                  isDimmed={isEdgeDimmed(e)}
                  isFocused={isCycleEdge(e)}
                  isHovered={props.hoveredEdge?.i === e.i}
                  isInteractive={isEdgeInteractive()}
                  onHover={props.onHoverEdge}
                />
              </Show>
            )}
          </For>
        </g>

        <g class="modules">
          <For each={renderedModules()}>
            {([id, pos]) => (
              <ModuleNode
                id={id}
                pos={pos}
                hue={hueByContainer().get(pos.mod.container)}
                isDimmed={isModuleDimmed(id)}
                isHighlighted={highlight() === id}
                isSelected={props.selectedId === id}
                isHaloThick={props.isHaloThick}
                colorMode={props.colorMode}
                onSelect={props.onSelect}
                onHover={props.onHover}
              />
            )}
          </For>
        </g>
      </g>
    </g>
  )
}
