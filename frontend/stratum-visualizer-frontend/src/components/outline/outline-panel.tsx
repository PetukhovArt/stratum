import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { isModuleFilterActive, moduleMatcher, toggled } from '../../lib/filters'
import { ancestorIds, DesignContainer, DesignData, DesignLayer, DesignModule } from '../../render/design'
import { layerColor, TONE } from '../../render/palette'
import { Filters } from '../../state'
import { Chevron } from './chevron'
import { ModuleRow } from './module-row'
import { OutlineContext, OutlineContextValue, useOutline } from './outline-context'
import { layerCountTip, scopeViolationsTip } from './tip-content'
import { createTip, TreeTooltip } from './tree-tooltip'

const visibleOutlineModules = (data: DesignData, f: Filters): Set<string> | null => {
  if (!isModuleFilterActive(f)) return null
  const matches = moduleMatcher(f, data)
  const direct = data.modules.filter(matches).map((m) => m.id)
  const visible = new Set<string>(direct)
  for (const id of direct) {
    for (const a of ancestorIds(data.moduleById, id)) visible.add(a)
  }
  const queue = [...direct]
  while (queue.length) {
    const id = queue.pop() as string
    for (const kid of data.childIndex[id] ?? []) {
      if (!visible.has(kid)) {
        visible.add(kid)
        queue.push(kid)
      }
    }
  }
  return visible
}

const SeverityPills: Component<{ errors: number; warnings: number; scope: () => DesignModule[]; label: string }> = (
  props,
) => {
  const outline = useOutline()
  const pill = (severity: 'error' | 'warning', count: () => number, cls: string) => (
    <Show when={count() > 0}>
      <span
        class={`pill ${cls}`}
        onMouseEnter={(e) =>
          outline.tip.show(e, () =>
            scopeViolationsTip(outline.data, { modules: props.scope(), label: props.label, severity }),
          )
        }
        onMouseMove={outline.tip.move}
        onMouseLeave={outline.tip.hide}
      >
        {count()}
      </span>
    </Show>
  )
  return (
    <>
      {pill('error', () => props.errors, 'pill-err')}
      {pill('warning', () => props.warnings, 'pill-warn')}
    </>
  )
}

const EyeToggle: Component<{ isHidden: boolean; onToggle: () => void; title?: string }> = (props) => (
  <button
    class={`ot-eye ${props.isHidden ? 'off' : ''}`}
    onClick={(e) => {
      e.stopPropagation()
      props.onToggle()
    }}
    title={props.title}
  >
    {props.isHidden ? '○' : '●'}
  </button>
)

const ContainerRow: Component<{
  container: DesignContainer
  filters: Filters
  setFilters: (f: Filters) => void
}> = (props) => {
  const outline = useOutline()
  const c = () => props.container
  const tops = () => (outline.data.topByContainer[c().id] ?? []).filter(outline.isModuleVisible)
  const containerModules = createMemo(() => outline.data.modules.filter((m) => m.container === c().id))
  return (
    <div class="ot-container">
      <div class="ot-row ot-row-ctr" onClick={() => outline.toggle(c().id)}>
        <Chevron isCollapsed={outline.isCollapsed(c().id)} />
        <span class="ot-name ot-mono">{c().label}</span>
        <span class="ot-meta">
          <SeverityPills
            errors={c().errors}
            warnings={c().warnings}
            scope={containerModules}
            label={c().label}
          />
        </span>
        <EyeToggle
          isHidden={props.filters.disabledContainers.has(c().id)}
          onToggle={() =>
            props.setFilters({
              ...props.filters,
              disabledContainers: toggled(props.filters.disabledContainers, c().id),
            })
          }
        />
      </div>
      <Show when={!outline.isCollapsed(c().id)}>
        <For each={tops()}>{(modId) => <ModuleRow modId={modId} depth={0} />}</For>
      </Show>
    </div>
  )
}

const LayerRow: Component<{
  layer: DesignLayer
  containers: DesignContainer[]
  filters: Filters
  setFilters: (f: Filters) => void
}> = (props) => {
  const outline = useOutline()
  const layer = () => props.layer
  const isHidden = () => props.filters.disabledLayers.has(layer().id)
  const layerModules = createMemo(() =>
    outline.data.modules.filter((m) => outline.data.layerByContainer.get(m.container)?.id === layer().id),
  )
  return (
    <div class="ot-layer">
      <div class="ot-row ot-row-layer" onClick={() => outline.toggle(layer().id)}>
        <Chevron isCollapsed={outline.isCollapsed(layer().id)} />
        <span class="ot-dot" style={{ background: layerColor(layer().hue, TONE.dot) }} />
        <span class="ot-name">{layer().label}</span>
        <span class="ot-meta">
          <SeverityPills
            errors={layer().errors}
            warnings={layer().warnings}
            scope={layerModules}
            label={`${layer().label} layer`}
          />
          <span
            class="ot-num"
            onMouseEnter={(e) => outline.tip.show(e, () => layerCountTip(layer()))}
            onMouseMove={outline.tip.move}
            onMouseLeave={outline.tip.hide}
          >
            {layer().modules}
          </span>
        </span>
        <EyeToggle
          isHidden={isHidden()}
          title={isHidden() ? 'show layer' : 'hide layer'}
          onToggle={() =>
            props.setFilters({
              ...props.filters,
              disabledLayers: toggled(props.filters.disabledLayers, layer().id),
            })
          }
        />
      </div>
      <Show when={!outline.isCollapsed(layer().id)}>
        <For each={props.containers}>
          {(c) => <ContainerRow container={c} filters={props.filters} setFilters={props.setFilters} />}
        </For>
      </Show>
    </div>
  )
}

export const OutlinePanel: Component<{
  data: DesignData
  filters: Filters
  setFilters: (f: Filters) => void
  selectedId: string | null
  onSelect: (id: string) => void
}> = (props) => {
  const [collapsed, setCollapsed] = createSignal<Set<string>>(
    new Set(props.data.modules.filter((m) => m.depth >= 1).map((m) => m.id)),
  )
  const tip = createTip()

  const visible = createMemo(() => visibleOutlineModules(props.data, props.filters))
  const isFiltering = () => visible() !== null
  const isModuleVisible = (id: string): boolean => visible()?.has(id) ?? true
  const visibleCount = createMemo(() => visible()?.size ?? props.data.modules.length)

  const shownContainers = createMemo(() => {
    const v = visible()
    if (!v) return props.data.containers
    const withVisible = new Set(props.data.modules.filter((m) => v.has(m.id)).map((m) => m.container))
    return props.data.containers.filter((c) => withVisible.has(c.id))
  })
  const containersOf = (layerId: string) => shownContainers().filter((c) => c.layer === layerId)

  const outline: OutlineContextValue = {
    data: props.data,
    tip: tip.controller,
    selectedId: () => props.selectedId,
    onSelect: (id) => props.onSelect(id),
    toggle: (id) => setCollapsed(toggled(collapsed(), id)),
    isCollapsed: (id) => !isFiltering() && collapsed().has(id),
    isModuleVisible,
  }

  return (
    <OutlineContext.Provider value={outline}>
      <aside class="panel panel-outline">
        <header class="panel-hd">
          <span class="panel-title">Outline</span>
          <span class="panel-count">
            {isFiltering() ? `${visibleCount()} / ${props.data.modules.length}` : props.data.modules.length}
          </span>
        </header>
        <div class="outline-tree" onScroll={tip.controller.hide}>
          <For each={props.data.layers}>
            {(layer) => (
              <Show when={!isFiltering() || containersOf(layer.id).length > 0}>
                <LayerRow
                  layer={layer}
                  containers={containersOf(layer.id)}
                  filters={props.filters}
                  setFilters={props.setFilters}
                />
              </Show>
            )}
          </For>
        </div>
        <TreeTooltip render={tip.render()} point={tip.point()} />
      </aside>
    </OutlineContext.Provider>
  )
}
