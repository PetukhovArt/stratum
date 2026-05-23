import { Component, createMemo, For, onCleanup, onMount, Show } from 'solid-js'
import { Portal } from 'solid-js/web'
import { DesignData, DesignViolation } from '../render/design'

interface ModuleCardProps {
  data: DesignData
  moduleId: string
  onClose: () => void
  onSelect: (id: string) => void
  onFocusCycle?: (vid: string) => void
}

/**
 * Floating HTML overlay shown when the user clicks a small (sub-readable)
 * leaf module in the WebGL canvas. Renders the same data as DetailsPanel,
 * sized as a centered card, dismissed via × / Esc / backdrop click.
 */
export const ModuleCard: Component<ModuleCardProps> = (props) => {
  const mod = createMemo(() => props.data.modules.find((m) => m.id === props.moduleId) ?? null)
  const container = createMemo(() => {
    const m = mod()
    return m ? props.data.containers.find((c) => c.id === m.container) ?? null : null
  })
  const layer = createMemo(() => {
    const c = container()
    return c ? props.data.layers.find((l) => l.id === c.layer) ?? null : null
  })
  const incoming = createMemo(() =>
    mod() ? props.data.edges.filter((e) => e.target === props.moduleId) : [],
  )
  const outgoing = createMemo(() =>
    mod() ? props.data.edges.filter((e) => e.source === props.moduleId) : [],
  )
  const vios = createMemo(() => {
    const m = mod()
    if (!m) return [] as DesignViolation[]
    return m.violations
      .map((vid) => props.data.violations.find((v) => v.id === vid))
      .filter((v): v is DesignViolation => !!v)
  })
  const kids = createMemo(() =>
    mod() ? props.data.childIndex[props.moduleId] ?? [] : ([] as string[]),
  )

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation()
        props.onClose()
      }
    }
    window.addEventListener('keydown', onKey, { capture: true })
    onCleanup(() => window.removeEventListener('keydown', onKey, { capture: true } as never))
  })

  return (
    <Show when={mod()}>
      {(m) => (
        <Portal>
          <div class="modcard-backdrop" onClick={props.onClose}>
            <div class="modcard" onClick={(e) => e.stopPropagation()}>
              <header class="modcard-hd">
                <div class="modcard-hd-l">
                  <Show when={layer()}>
                    {(l) => (
                      <span
                        class="hd-pill"
                        style={{
                          background: `oklch(70% 0.10 ${l().hue} / .2)`,
                          color: `oklch(82% 0.10 ${l().hue})`,
                        }}
                      >
                        {l().label}
                      </span>
                    )}
                  </Show>
                  <Show when={m().kind === 'composer'}>
                    <span class="hd-pill pill-kind-c">◇ composer</span>
                  </Show>
                  <Show when={m().kind === 'shared'}>
                    <span class="hd-pill pill-kind-s">_shared</span>
                  </Show>
                  <Show when={m().severity}>
                    <span class={`hd-pill sev-${m().severity}`}>{m().severity}</span>
                  </Show>
                  <span class="modcard-title">{m().label}</span>
                </div>
                <button class="modcard-close" onClick={props.onClose}>
                  ×
                </button>
              </header>

              <div class="modcard-body">
                <div class="modcard-path">{m().path}</div>

                <div class="modcard-stats">
                  <Stat label="LoC" value={m().loc || '—'} />
                  <Stat label="Stage" value={m().stage} />
                  <Stat label="In" value={incoming().length} />
                  <Stat label="Out" value={outgoing().length} />
                  <Show when={kids().length > 0}>
                    <Stat label="Kids" value={kids().length} />
                  </Show>
                </div>

                <Show when={vios().length > 0}>
                  <section class="modcard-section">
                    <h4>Violations · {vios().length}</h4>
                    <div class="vios">
                      <For each={vios()}>
                        {(v) => (
                          <div class={`vio vio-${v.severity}`}>
                            <div class="vio-hd">
                              <span class={`vio-sev sev-${v.severity}`}>{v.severity}</span>
                              <span class="vio-rule">{v.rule}</span>
                              <Show when={v.rule.includes('cycle') || v.rule.includes('circular')}>
                                <button
                                  class="vio-focus"
                                  onClick={() => {
                                    props.onFocusCycle?.(v.id)
                                    props.onClose()
                                  }}
                                >
                                  ⊙ focus
                                </button>
                              </Show>
                            </div>
                            <div class="vio-msg">{v.message}</div>
                            <div class="vio-doc">{v.ruleDoc}</div>
                          </div>
                        )}
                      </For>
                    </div>
                  </section>
                </Show>

                <Show when={kids().length > 0}>
                  <section class="modcard-section">
                    <h4>Children · {kids().length}</h4>
                    <div class="kidgrid">
                      <For each={kids()}>
                        {(kid) => {
                          const k = props.data.modules.find((mm) => mm.id === kid)
                          if (!k) return null
                          return (
                            <div
                              class={`kidgrid-item kind-${k.kind}`}
                              onClick={() => {
                                props.onSelect(kid)
                                props.onClose()
                              }}
                            >
                              <div class="kidgrid-name">
                                <Show when={k.kind === 'composer'}>
                                  <span class="kidgrid-tag">◇</span>
                                </Show>
                                <Show when={k.kind === 'shared'}>
                                  <span class="kidgrid-tag">_</span>
                                </Show>
                                {k.label}
                              </div>
                              <div class="kidgrid-loc">
                                {k.loc ? `${k.loc} LoC · ` : ''}stage {k.stage}
                              </div>
                            </div>
                          )
                        }}
                      </For>
                    </div>
                  </section>
                </Show>

                <Show when={outgoing().length > 0}>
                  <section class="modcard-section">
                    <h4>Outgoing · {outgoing().length}</h4>
                    <For each={outgoing().slice(0, 12)}>
                      {(e) => (
                        <EdgeRow
                          dir="→"
                          kind={e.kind}
                          violation={e.violation}
                          otherId={e.target}
                          data={props.data}
                          onClick={() => {
                            props.onSelect(e.target)
                            props.onClose()
                          }}
                        />
                      )}
                    </For>
                    <Show when={outgoing().length > 12}>
                      <div class="modcard-more">+{outgoing().length - 12} more</div>
                    </Show>
                  </section>
                </Show>

                <Show when={incoming().length > 0}>
                  <section class="modcard-section">
                    <h4>Incoming · {incoming().length}</h4>
                    <For each={incoming().slice(0, 12)}>
                      {(e) => (
                        <EdgeRow
                          dir="←"
                          kind={e.kind}
                          violation={e.violation}
                          otherId={e.source}
                          data={props.data}
                          onClick={() => {
                            props.onSelect(e.source)
                            props.onClose()
                          }}
                        />
                      )}
                    </For>
                    <Show when={incoming().length > 12}>
                      <div class="modcard-more">+{incoming().length - 12} more</div>
                    </Show>
                  </section>
                </Show>
              </div>
            </div>
          </div>
        </Portal>
      )}
    </Show>
  )
}

const Stat: Component<{ label: string; value: number | string }> = (p) => (
  <div class="stat">
    <div class="stat-val">{p.value}</div>
    <div class="stat-lbl">{p.label}</div>
  </div>
)

const EdgeRow: Component<{
  dir: '→' | '←'
  kind: 'static' | 'di' | 'runtime'
  violation: string | null
  otherId: string
  data: DesignData
  onClick: () => void
}> = (props) => {
  const other = () => props.data.modules.find((m) => m.id === props.otherId)
  return (
    <Show when={other()}>
      {(o) => (
        <div
          class={`edge-row ${props.violation ? 'edge-row-vio' : ''}`}
          onClick={props.onClick}
        >
          <span class="edge-row-dir">{props.dir}</span>
          <span class="edge-row-kind" data-kind={props.kind}>
            {props.kind}
          </span>
          <span class="edge-row-name">{o().label}</span>
        </div>
      )}
    </Show>
  )
}
