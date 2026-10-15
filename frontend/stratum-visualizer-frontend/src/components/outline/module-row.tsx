import { Component, For, Show } from 'solid-js'
import { Chevron } from './chevron'
import { useOutline } from './outline-context'
import { moduleViolationsTip, nestedCountTip } from './tip-content'

const ROW_INDENT_BASE_PX = 28
const ROW_INDENT_STEP_PX = 14

export const ModuleRow: Component<{ modId: string; depth: number }> = (props) => {
  const outline = useOutline()
  const mod = () => outline.data.moduleById.get(props.modId)
  const kids = () => (outline.data.childIndex[props.modId] ?? []).filter(outline.isModuleVisible)
  const hasKids = () => kids().length > 0
  const isCollapsed = () => outline.isCollapsed(props.modId)

  return (
    <Show when={mod()}>
      {(m) => (
        <>
          <div
            class={`ot-row ot-row-mod ${outline.selectedId() === props.modId ? 'sel' : ''} kind-${m().kind}`}
            style={{ 'padding-left': `${ROW_INDENT_BASE_PX + props.depth * ROW_INDENT_STEP_PX}px` }}
            onClick={() => outline.onSelect(props.modId)}
          >
            <Show when={hasKids()} fallback={<span class="ot-chev" aria-hidden="true" />}>
              <Chevron
                isCollapsed={isCollapsed()}
                onClick={(e) => {
                  e.stopPropagation()
                  outline.toggle(props.modId)
                }}
              />
            </Show>
            <Show when={m().kind === 'composer'}>
              <span class="ot-kind ot-kind-c">◇</span>
            </Show>
            <Show when={m().kind === 'shared'}>
              <span class="ot-kind ot-kind-s">_</span>
            </Show>
            <span class="ot-mono ot-mod-name">{m().label}</span>
            <Show when={m().severity === 'error' || m().severity === 'warning'}>
              <span
                class={`ot-mod-sev ${m().severity === 'error' ? 'sev-err' : 'sev-warn'}`}
                onMouseEnter={(e) => outline.tip.show(e, () => moduleViolationsTip(outline.data, m()))}
                onMouseMove={outline.tip.move}
                onMouseLeave={outline.tip.hide}
              />
            </Show>
            <Show when={hasKids()}>
              <span
                class="ot-mod-thick"
                onMouseEnter={(e) => outline.tip.show(e, () => nestedCountTip(kids().length))}
                onMouseMove={outline.tip.move}
                onMouseLeave={outline.tip.hide}
              >
                {kids().length}
              </span>
            </Show>
          </div>
          <Show when={hasKids() && !isCollapsed()}>
            <For each={kids()}>{(kidId) => <ModuleRow modId={kidId} depth={props.depth + 1} />}</For>
          </Show>
        </>
      )}
    </Show>
  )
}
