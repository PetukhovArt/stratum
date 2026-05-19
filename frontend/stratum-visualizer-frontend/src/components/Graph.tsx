import { Component, createMemo, For, onCleanup, onMount, Show } from 'solid-js'
import { DesignData, DesignModule } from '../render/design'
import { LaidEdge, LaneInfo, ContainerInfo, ModulePos, Scene } from '../render/layout'
import { Filters, HoveredEdge, HoveredMod, Tweaks, Viewport } from '../state'

const layerColor = (hue: number, alpha = 1, l = 60, c = 0.1) =>
  `oklch(${l}% ${c} ${hue} / ${alpha})`

interface Severity {
  stroke: string
  fill: string
  glow: string
}

const SEVERITY: Record<string, Severity> = {
  error: { stroke: 'oklch(64% 0.18 25)', fill: 'oklch(64% 0.18 25 / .14)', glow: 'oklch(64% 0.18 25 / .35)' },
  warning: { stroke: 'oklch(78% 0.14 70)', fill: 'oklch(78% 0.14 70 / .14)', glow: 'oklch(78% 0.14 70 / .30)' },
  info: { stroke: 'oklch(70% 0.10 230)', fill: 'oklch(70% 0.10 230 / .14)', glow: 'oklch(70% 0.10 230 / .30)' },
}

// ─── glob → regex ───────────────────────────────────────────────────────────

export const globToRegex = (glob: string): ((p: string) => boolean) | null => {
  if (!glob) return null
  const parts = glob
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
  const regexes = parts.map((p) => {
    const negate = p.startsWith('!')
    const pat = negate ? p.slice(1) : p
    const re =
      '^' +
      pat
        .replace(/[.+^${}()|[\]\\]/g, '\\$&')
        .replace(/\*\*/g, '§§DSTAR§§')
        .replace(/\*/g, '[^/]*')
        .replace(/§§DSTAR§§/g, '.*')
        .replace(/\?/g, '.') +
      '$'
    return { negate, re: new RegExp(re) }
  })
  return (path: string) => {
    let matched = false
    let anyPositive = false
    for (const { negate, re } of regexes) {
      if (!negate) anyPositive = true
      if (re.test(path)) {
        matched = !negate
      }
    }
    if (!anyPositive) return !matched
    return matched
  }
}

const buildContainerLayerMap = (scene: Scene): Map<string, string> => {
  const m = new Map<string, string>()
  for (const c of scene.containers) m.set(c.id, c.layer)
  return m
}

const buildVisibility = (
  scene: Scene,
  filters: Filters,
  containerLayer: Map<string, string>,
): Set<string> => {
  const matchGlob = filters.glob ? globToRegex(filters.glob) : null
  const matchSet = new Set<string>()
  for (const [id, pos] of Object.entries(scene.modulePos)) {
    const mod = pos.mod
    const lay = containerLayer.get(mod.container) ?? ''
    if (filters.disabledLayers.has(lay)) continue
    if (filters.disabledContainers.has(mod.container)) continue
    if (
      filters.onlyViolators &&
      mod.violations.length === 0 &&
      mod.descError === 0 &&
      mod.descWarn === 0
    )
      continue
    if (filters.stageFilter !== null && mod.stage !== filters.stageFilter) continue
    if (filters.query) {
      const q = filters.query.toLowerCase()
      if (!mod.id.toLowerCase().includes(q) && !mod.path.toLowerCase().includes(q)) continue
    }
    if (matchGlob && !matchGlob(mod.path)) continue
    matchSet.add(id)
  }
  const visible = new Set<string>(matchSet)
  for (const id of matchSet) {
    const pos = scene.modulePos[id]
    if (!pos) continue
    let cur: string | null = pos.mod.parentId
    while (cur) {
      visible.add(cur)
      const p = scene.modulePos[cur]
      if (!p) break
      cur = p.mod.parentId
    }
  }
  return visible
}

interface GraphProps {
  scene: Scene
  data: DesignData
  filters: Filters
  tweaks: Tweaks
  selectedId: string | null
  hoveredId: string | null
  focusedCycle: string | null
  onSelect: (id: string) => void
  onHover: (h: HoveredMod | null) => void
  onHoverEdge: (h: HoveredEdge | null) => void
  hoveredEdge: HoveredEdge | null
  viewport: Viewport
  onViewportChange: (v: Viewport) => void
}

export const Graph: Component<GraphProps> = (props) => {
  let svgRef!: SVGSVGElement

  const containerLayer = createMemo(() => buildContainerLayerMap(props.scene))
  const visibleMods = createMemo(() =>
    buildVisibility(props.scene, props.filters, containerLayer()),
  )

  // Pre-index edges by module so neighbour/connected checks become O(1).
  const edgesByMod = createMemo(() => {
    const m = new Map<string, LaidEdge[]>()
    for (const e of props.scene.edges) {
      ;(m.get(e.source) ?? m.set(e.source, []).get(e.source)!).push(e)
      ;(m.get(e.target) ?? m.set(e.target, []).get(e.target)!).push(e)
    }
    return m
  })

  const cycleEdgeIds = createMemo(() => {
    if (!props.focusedCycle) return null
    const ids = new Set<number>()
    for (const e of props.scene.edges) if (e.violation === props.focusedCycle) ids.add(e.i)
    return ids
  })

  const cycleModIds = createMemo(() => {
    if (!props.focusedCycle) return null
    const ids = new Set<string>()
    for (const e of props.scene.edges) {
      if (e.violation === props.focusedCycle) {
        ids.add(e.source)
        ids.add(e.target)
      }
    }
    return ids
  })

  const highlight = () => props.hoveredId ?? props.selectedId

  const neighbours = createMemo(() => {
    const h = highlight()
    if (!h) return null
    const set = new Set<string>([h])
    for (const e of edgesByMod().get(h) ?? []) {
      set.add(e.source === h ? e.target : e.source)
    }
    const pos = props.scene.modulePos[h]
    if (pos) {
      let cur: string | null = pos.mod.parentId
      while (cur) {
        set.add(cur)
        const p = props.scene.modulePos[cur]
        if (!p) break
        cur = p.mod.parentId
      }
    }
    return set
  })

  const dim = (modId: string): boolean => {
    const cy = cycleModIds()
    if (cy) return !cy.has(modId)
    if (!visibleMods().has(modId)) return true
    const h = highlight()
    const nb = neighbours()
    if (h && nb && !nb.has(modId)) return true
    return false
  }

  const edgeVisible = (e: LaidEdge): boolean => {
    const cy = cycleEdgeIds()
    if (cy) return cy.has(e.i)
    if (props.filters.onlyViolators && !e.violation) return false
    if (!visibleMods().has(e.source) || !visibleMods().has(e.target)) return false
    if (!props.filters.edgeKinds.has(e.kind)) return false
    // Connection density gate: in `minimal` mode (default), only paint violation edges
    // + edges touching the hovered/selected module. Saves ~5000 SVG nodes on real
    // projects without losing the "where are the problems?" signal.
    if (props.filters.connectionMode === 'minimal' && !e.violation) {
      const h = highlight()
      if (!h || (e.source !== h && e.target !== h)) return false
    }
    return true
  }

  const edgeDim = (e: LaidEdge): boolean => {
    if (props.focusedCycle) return false
    const h = highlight()
    // When something's hovered/selected, dim everything not adjacent — including
    // violation edges. Otherwise non-adjacent violations dominate the canvas and
    // drown out the green neighbour highlight the user came to inspect.
    if (h && e.source !== h && e.target !== h) return true
    return false
  }

  let drag: { x: number; y: number; vx: number; vy: number } | null = null

  const onWheel = (e: WheelEvent) => {
    e.preventDefault()
    const r = svgRef.getBoundingClientRect()
    const mx = e.clientX - r.left
    const my = e.clientY - r.top
    const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12
    const v = props.viewport
    const newZoom = Math.max(0.15, Math.min(4, v.zoom * factor))
    const wx = (mx - v.x) / v.zoom
    const wy = (my - v.y) / v.zoom
    props.onViewportChange({ zoom: newZoom, x: mx - wx * newZoom, y: my - wy * newZoom })
  }
  const onMouseDown = (e: MouseEvent) => {
    const t = e.target as Element
    if (t.closest('[data-mod-id]') || t.closest('[data-edge-i]')) return
    drag = { x: e.clientX, y: e.clientY, vx: props.viewport.x, vy: props.viewport.y }
  }
  const onMouseMove = (e: MouseEvent) => {
    if (!drag) return
    const dx = e.clientX - drag.x
    const dy = e.clientY - drag.y
    props.onViewportChange({ ...props.viewport, x: drag.vx + dx, y: drag.vy + dy })
  }
  const onMouseUp = () => {
    drag = null
  }

  onMount(() => {
    svgRef.addEventListener('wheel', onWheel, { passive: false })
    onCleanup(() => svgRef.removeEventListener('wheel', onWheel))
  })

  const transformAttr = createMemo(
    () => `translate(${props.viewport.x} ${props.viewport.y}) scale(${props.viewport.zoom})`,
  )

  // depth-sorted modules so parents render below children
  const sortedMods = createMemo(() => {
    return Object.entries(props.scene.modulePos).sort((a, b) => a[1].depth - b[1].depth)
  })

  return (
    <svg
      ref={svgRef!}
      class="strat-graph"
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
      onMouseLeave={onMouseUp}
    >
      <defs>
        <marker id="arrow-default" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="5" markerHeight="5" orient="auto">
          <path d="M0,0 L8,4 L0,8 z" fill="currentColor" />
        </marker>
        <marker id="arrow-err" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto">
          <path d="M0,0 L8,4 L0,8 z" fill="oklch(64% 0.18 25)" />
        </marker>
        <marker id="arrow-warn" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto">
          <path d="M0,0 L8,4 L0,8 z" fill="oklch(78% 0.14 70)" />
        </marker>
        <marker id="arrow-out" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto">
          <path d="M0,0 L8,4 L0,8 z" fill="oklch(75% 0.16 150)" />
        </marker>
        <marker id="arrow-in" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto">
          <path d="M0,0 L8,4 L0,8 z" fill="oklch(78% 0.14 200)" />
        </marker>
        <pattern id="shared-hatch" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="oklch(72% 0.08 290 / .35)" stroke-width="1.2" />
        </pattern>
      </defs>

      <g transform={transformAttr()}>
        <Show when={props.tweaks.layerMode !== 'none'}>
          <For each={props.scene.lanes}>
            {(lane) => (
              <Lane
                lane={lane}
                tweaks={props.tweaks}
                dimmed={props.filters.disabledLayers.has(lane.id)}
                sceneW={props.scene.width}
              />
            )}
          </For>
        </Show>

        <g class="containers">
          <For each={props.scene.containers}>
            {(c) => (
              <ContainerBox
                container={c}
                tweaks={props.tweaks}
                layer={props.scene.lanes.find((l) => l.id === c.layer)}
                dimmed={
                  props.filters.disabledContainers.has(c.id) ||
                  props.filters.disabledLayers.has(c.layer)
                }
              />
            )}
          </For>
        </g>

        <g class="edges">
          <For each={props.scene.edges}>
            {(e) => (
              <Show when={edgeVisible(e)}>
                <EdgeLine
                  edge={e}
                  dim={edgeDim(e)}
                  tweaks={props.tweaks}
                  focused={!!cycleEdgeIds() && !!cycleEdgeIds()?.has(e.i)}
                  connected={!!highlight() && (e.source === highlight() || e.target === highlight())}
                  direction={
                    highlight() && e.source === highlight()
                      ? 'out'
                      : highlight() && e.target === highlight()
                        ? 'in'
                        : null
                  }
                  onHover={props.onHoverEdge}
                  hovered={!!props.hoveredEdge && props.hoveredEdge.i === e.i}
                  interactive={props.scene.edges.length <= 500}
                />
              </Show>
            )}
          </For>
        </g>

        <g class="modules">
          <For each={sortedMods()}>
            {([id, pos]) => (
              <ModuleNode
                id={id}
                pos={pos}
                dimmed={dim(id)}
                highlighted={highlight() === id}
                neighboured={
                  !!neighbours() && (neighbours() as Set<string>).has(id) && id !== highlight()
                }
                selected={props.selectedId === id}
                tweaks={props.tweaks}
                layer={props.scene.lanes.find((l) => l.id === containerLayerOf(pos, props.scene))}
                onSelect={props.onSelect}
                onHover={props.onHover}
              />
            )}
          </For>
        </g>
      </g>
    </svg>
  )
}

const containerLayerOf = (pos: ModulePos, scene: Scene): string => {
  const c = scene.containers.find((c) => c.id === pos.mod.container)
  return c?.layer ?? ''
}

// ─── Lane ───────────────────────────────────────────────────────────────────

const Lane: Component<{ lane: LaneInfo; tweaks: Tweaks; dimmed: boolean; sceneW: number }> = (
  props,
) => {
  return (
    <Show
      when={props.tweaks.layerMode !== 'borders'}
      fallback={
        <g opacity={props.dimmed ? 0.25 : 1} transform={`translate(0, ${props.lane.y})`}>
          <text
            x={4}
            y={18}
            class="lane-label-text"
            fill={layerColor(props.lane.hue, 0.95, 78, 0.08)}
          >
            {props.lane.label.toUpperCase()}
          </text>
          <text x={4} y={34} class="lane-count-text">
            {props.lane.modules} modules
          </text>
        </g>
      }
    >
      <g opacity={props.dimmed ? 0.25 : 1}>
        <rect
          x={0}
          y={props.lane.y}
          width={props.sceneW}
          height={props.lane.height}
          fill={layerColor(props.lane.hue, 0.045, 60, 0.18)}
          stroke="rgba(255,255,255,0.04)"
          stroke-width={1}
          rx={8}
        />
        <rect
          x={0}
          y={props.lane.y}
          width={4}
          height={props.lane.height}
          fill={layerColor(props.lane.hue, 0.7, 70, 0.2)}
          rx={2}
        />
        <text
          x={14}
          y={props.lane.y + 22}
          class="lane-label-text"
          fill={layerColor(props.lane.hue, 0.95, 82, 0.12)}
        >
          {props.lane.label.toUpperCase()}
        </text>
        <text x={14} y={props.lane.y + 38} class="lane-count-text">
          {props.lane.modules} modules{' '}
          {props.lane.errors > 0 ? `· ${props.lane.errors} err` : '·'}{' '}
          {props.lane.warnings > 0 ? `· ${props.lane.warnings} warn` : ''}
        </text>
      </g>
    </Show>
  )
}

// ─── Container compound ────────────────────────────────────────────────────

const STAGE_DESC: Record<number, string> = {
  1: 'Stage 1 · file — a single file, ≤2 exports, ~50–400 LOC. Smallest unit.',
  2: 'Stage 2 · folder — flat folder + index.ts, 2–6 files of one theme.',
  3: 'Stage 3 · segments — folder split into model / api / ui / lib.',
  4: 'Stage 4 · composed — composer + ≥2 sub-features + _shared. Fractal.',
}

const STAGE_COLORS = [
  '#666',
  'oklch(74% 0.12 220)',
  'oklch(74% 0.12 150)',
  'oklch(74% 0.12 95)',
  'oklch(78% 0.14 48)',
]

const ContainerBox: Component<{
  container: ContainerInfo
  layer: LaneInfo | undefined
  tweaks: Tweaks
  dimmed: boolean
}> = (props) => {
  const tone = () => layerColor(props.layer?.hue ?? 240, 0.05, 65, 0.1)
  const stroke = () =>
    props.tweaks.layerMode === 'borders'
      ? layerColor(props.layer?.hue ?? 240, 0.75, 70, 0.16)
      : 'rgba(255,255,255,0.08)'
  const stageColor = () => STAGE_COLORS[props.container.stage] ?? '#666'
  const c = () => props.container

  return (
    <g
      transform={`translate(${c().absX}, ${c().absY})`}
      opacity={props.dimmed ? 0.18 : 1}
    >
      <rect
        x={0}
        y={0}
        width={c().width}
        height={c().height}
        rx={10}
        fill={tone()}
        stroke={stroke()}
        stroke-width={props.tweaks.layerMode === 'borders' ? 1.2 : 0.8}
      />
      <g>
        <title>{STAGE_DESC[c().stage] ?? `Stage ${c().stage}`}</title>
        <rect
          x={8}
          y={5}
          width={14}
          height={11}
          rx={3}
          fill={stageColor()}
          fill-opacity={0.18}
          stroke={stageColor()}
          stroke-opacity={0.4}
          stroke-width={0.5}
        />
        <text
          x={15}
          y={13}
          class="container-stage-num"
          fill={stageColor()}
          text-anchor="middle"
        >
          {c().stage}
        </text>
        <rect x={6} y={3} width={18} height={15} fill="transparent" />
      </g>
      <text x={28} y={14} class="container-label">
        {c().label}
      </text>
      <Show when={c().errors > 0 || c().warnings > 0}>
        <g>
          <title>
            {[
              c().errors > 0 ? `${c().errors} error${c().errors === 1 ? '' : 's'}` : '',
              c().warnings > 0 ? `${c().warnings} warning${c().warnings === 1 ? '' : 's'}` : '',
            ]
              .filter(Boolean)
              .join('\n')}
          </title>
          <rect x={c().width - 60} y={3} width={55} height={14} fill="transparent" />
          <text x={c().width - 10} y={14} class="container-count" text-anchor="end">
            <Show when={c().errors > 0}>
              <tspan fill="oklch(64% 0.18 25)">{c().errors}e </tspan>
            </Show>
            <Show when={c().warnings > 0}>
              <tspan fill="oklch(78% 0.14 70)">{c().warnings}w</tspan>
            </Show>
          </text>
        </g>
      </Show>
    </g>
  )
}

// ─── Module Node ────────────────────────────────────────────────────────────

const STAGE_HUE = [220, 150, 95, 48]

const ModuleNode: Component<{
  id: string
  pos: ModulePos
  dimmed: boolean
  highlighted: boolean
  neighboured: boolean
  selected: boolean
  tweaks: Tweaks
  layer: LaneInfo | undefined
  onSelect: (id: string) => void
  onHover: (h: HoveredMod | null) => void
}> = (props) => {
  const mod = (): DesignModule => props.pos.mod
  const isCompound = () => !props.pos.leaf
  const hue = () =>
    props.tweaks.colorMode === 'stage'
      ? STAGE_HUE[(mod().stage || 1) - 1] ?? 220
      : (props.layer?.hue ?? 240)

  const kind = () => mod().kind
  const isComposer = () => kind() === 'composer'
  const isShared = () => kind() === 'shared'
  const sev = () => (mod().severity ? SEVERITY[mod().severity!] : null)

  const stroke = () => {
    if (props.selected) return 'oklch(85% 0.16 65)'
    const s = sev()
    if (s) return s.stroke
    if (isComposer()) return layerColor(hue(), 0.9, 78, 0.16)
    if (isShared()) return 'oklch(72% 0.10 290 / .55)'
    return layerColor(hue(), 0.65, 72, 0.1)
  }
  const strokeW = () => (props.selected ? 2.2 : sev() ? 1.5 : isComposer() ? 1.4 : 0.7)
  const dashArr = () => (isComposer() ? '3 2' : undefined)
  const baseFill = () => {
    if (isCompound()) return layerColor(hue(), 1, 26, 0.08)
    if (isShared()) return 'url(#shared-hatch)'
    return layerColor(hue(), 1, 38, 0.1)
  }
  const rounding = () => (props.pos.depth === 0 ? 6 : 4)

  const onEnter = (e: MouseEvent) => props.onHover({ id: props.id, x: e.clientX, y: e.clientY })
  const onLeave = () => props.onHover(null)

  return (
    <g
      transform={`translate(${props.pos.x}, ${props.pos.y})`}
      data-mod-id={props.id}
      opacity={props.dimmed ? 0.1 : 1}
      style={{ cursor: 'pointer', transition: 'opacity 180ms ease' }}
      onMouseEnter={onEnter}
      onMouseLeave={onLeave}
      onClick={(e) => {
        e.stopPropagation()
        props.onSelect(props.id)
      }}
    >
      <Show when={isCompound() && props.tweaks.haloThick}>
        <rect
          x={-3}
          y={-3}
          width={props.pos.w + 6}
          height={props.pos.h + 6}
          rx={rounding() + 3}
          fill="none"
          stroke={layerColor(hue(), 0.4, 72, 0.12)}
          stroke-width={1}
          stroke-dasharray="2 3"
        />
      </Show>
      <Show when={sev()}>
        <rect
          x={-2}
          y={-2}
          width={props.pos.w + 4}
          height={props.pos.h + 4}
          rx={rounding() + 2}
          fill={sev()!.glow}
          opacity={0.55}
        />
      </Show>
      <rect
        x={0}
        y={0}
        width={props.pos.w}
        height={props.pos.h}
        rx={rounding()}
        fill={baseFill()}
        stroke={stroke()}
        stroke-width={strokeW()}
        stroke-dasharray={dashArr()}
      />
      <Show when={isComposer() && !isCompound()}>
        <path
          d={`M ${props.pos.w / 2} 1 L ${props.pos.w - 1} ${props.pos.h / 2} L ${props.pos.w / 2} ${props.pos.h - 1} L 1 ${props.pos.h / 2} z`}
          fill="none"
          stroke={layerColor(hue(), 0.95, 84, 0.18)}
          stroke-width={1}
        />
      </Show>
      <Show when={isShared() && !isCompound()}>
        <circle cx={props.pos.w - 3} cy={3} r={1.8} fill="oklch(78% 0.10 290 / .85)" />
      </Show>
      <Show when={props.highlighted}>
        <rect
          x={-3}
          y={-3}
          width={props.pos.w + 6}
          height={props.pos.h + 6}
          rx={rounding() + 3}
          fill="none"
          stroke="oklch(85% 0.16 65)"
          stroke-width={1.4}
          stroke-dasharray="2 3"
        />
      </Show>

      <Show when={isCompound()}>
        <text x={7} y={12} class={`mod-label depth-${props.pos.depth} kind-${kind()}`}>
          {isComposer() ? '◇ ' : ''}
          {mod().label}
        </text>
        <Show when={props.pos.depth === 0 && (mod().descError > 0 || mod().descWarn > 0)}>
          <g>
            <title>
              {[
                mod().descError > 0
                  ? `${mod().descError} error${mod().descError === 1 ? '' : 's'} in descendants`
                  : '',
                mod().descWarn > 0
                  ? `${mod().descWarn} warning${mod().descWarn === 1 ? '' : 's'} in descendants`
                  : '',
              ]
                .filter(Boolean)
                .join('\n')}
            </title>
            <rect x={props.pos.w - 50} y={2} width={48} height={14} fill="transparent" />
            <text x={props.pos.w - 6} y={12} class="mod-count" text-anchor="end">
              <Show when={mod().descError > 0}>
                <tspan fill="oklch(64% 0.18 25)">{mod().descError}e </tspan>
              </Show>
              <Show when={mod().descWarn > 0}>
                <tspan fill="oklch(78% 0.14 70)">{mod().descWarn}w</tspan>
              </Show>
            </text>
          </g>
        </Show>
      </Show>

      <Show when={!isCompound()}>
        <circle
          cx={props.pos.w - 3}
          cy={props.pos.h - 3}
          r={1.4}
          fill={STAGE_COLORS[mod().stage] ?? '#666'}
        />
      </Show>
    </g>
  )
}

// ─── Edge ──────────────────────────────────────────────────────────────────

const edgePath = (e: LaidEdge, style: Tweaks['edgeStyle']): string => {
  const dy = e.y2 - e.y1
  if (style === 'line') return `M${e.x1},${e.y1} L${e.x2},${e.y2}`
  if (style === 'ortho') {
    const midY = e.y1 + dy / 2
    return `M${e.x1},${e.y1} L${e.x1},${midY} L${e.x2},${midY} L${e.x2},${e.y2}`
  }
  const cx1 = e.x1
  const cy1 = e.y1 + dy * 0.4
  const cx2 = e.x2
  const cy2 = e.y2 - dy * 0.4
  return `M${e.x1},${e.y1} C${cx1},${cy1} ${cx2},${cy2} ${e.x2},${e.y2}`
}

const EdgeLine: Component<{
  edge: LaidEdge
  dim: boolean
  tweaks: Tweaks
  focused: boolean
  connected: boolean
  direction: 'in' | 'out' | null
  onHover: (h: HoveredEdge | null) => void
  hovered: boolean
  interactive: boolean
}> = (props) => {
  const path = createMemo(() => edgePath(props.edge, props.tweaks.edgeStyle))
  const sev = () => props.edge.violation

  const baseStroke = () => {
    if (sev()) return 'oklch(64% 0.18 25)'
    if (props.connected && props.direction === 'out') return 'oklch(75% 0.16 150)'
    if (props.connected && props.direction === 'in') return 'oklch(78% 0.14 200)'
    if (props.edge.kind === 'di') return 'oklch(72% 0.10 200)'
    if (props.edge.kind === 'runtime') return 'oklch(72% 0.10 290)'
    return 'oklch(72% 0.02 240)'
  }
  const dasharray = () => {
    if (props.edge.kind === 'di') return '4 3'
    if (props.edge.kind === 'runtime') return '1 4'
    if (sev()) return '6 4'
    return undefined
  }
  const opacity = () => {
    if (props.focused) return 1
    if (props.dim) return 0.04
    if (sev()) return 0.95
    if (props.connected) return 1
    if (props.hovered) return 0.95
    return 0.3
  }
  const sw = () => (sev() ? 1.7 : props.connected ? 1.6 : props.hovered ? 1.4 : 0.8)

  const marker = () => {
    if (sev()) return 'url(#arrow-err)'
    if (props.connected && props.direction === 'out') return 'url(#arrow-out)'
    if (props.connected && props.direction === 'in') return 'url(#arrow-in)'
    return 'url(#arrow-default)'
  }

  const onEnter = (e: MouseEvent) =>
    props.onHover({
      i: props.edge.i,
      source: props.edge.source,
      target: props.edge.target,
      kind: props.edge.kind,
      violation: props.edge.violation,
      x: e.clientX,
      y: e.clientY,
    })

  return (
    <g
      data-edge-i={props.edge.i}
      style={{ color: baseStroke() }}
      onMouseEnter={props.interactive ? onEnter : undefined}
      onMouseLeave={props.interactive ? () => props.onHover(null) : undefined}
    >
      <Show when={props.interactive}>
        <path
          d={path()}
          fill="none"
          stroke="transparent"
          stroke-width={10}
          style={{ 'pointer-events': 'stroke' }}
        />
      </Show>
      <Show when={props.connected && !sev()}>
        <path
          d={path()}
          fill="none"
          stroke={baseStroke()}
          stroke-width={4}
          opacity={0.25}
          style={{ filter: 'blur(2px)', 'pointer-events': 'none' }}
        />
      </Show>
      <path
        d={path()}
        fill="none"
        stroke={baseStroke()}
        stroke-width={sw()}
        stroke-dasharray={dasharray()}
        opacity={opacity()}
        marker-end={marker()}
        style={{ 'pointer-events': 'none', transition: 'opacity 120ms' }}
      />
      <Show when={sev()}>
        <path
          d={path()}
          fill="none"
          stroke="oklch(64% 0.18 25)"
          stroke-width={3.5}
          opacity={0.18}
          style={{ filter: 'blur(2px)', 'pointer-events': 'none' }}
        />
        <circle
          r={3}
          cx={(props.edge.x1 + props.edge.x2) / 2}
          cy={(props.edge.y1 + props.edge.y2) / 2}
          fill="oklch(64% 0.18 25)"
          class="pulse-dot"
          style={{ 'pointer-events': 'none' }}
        />
      </Show>
    </g>
  )
}
