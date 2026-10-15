import { Component, createMemo, For, JSX, Show } from 'solid-js'
import {
  ancestorIds,
  containerKindOf,
  DesignData,
  DesignModule,
  DesignViolation,
  EdgeKindLabel,
  layerOfModule,
  violationsOf,
} from '../render/design'
import { facetInfo, layerColor, TONE } from '../render/palette'

interface CycleFocus {
  focusedCycle: string | null
  onFocusCycle: (vid: string) => void
}

const ViolationCard: Component<{ violation: DesignViolation; cycle: CycleFocus }> = (props) => {
  const v = () => props.violation
  return (
    <div class={`vio vio-${v().severity}`}>
      <div class="vio-hd">
        <span class={`vio-sev sev-${v().severity}`}>{v().severity}</span>
        <span class="vio-rule">{v().rule}</span>
        <Show when={v().isCycle}>
          <button class="vio-focus" onClick={() => props.cycle.onFocusCycle(v().id)}>
            {props.cycle.focusedCycle === v().id ? '◉ focused' : '⊙ focus'}
          </button>
        </Show>
      </div>
      <div class="vio-msg">{v().message}</div>
    </div>
  )
}

const Section: Component<{ title: string; children: JSX.Element }> = (props) => (
  <section class="det-section">
    <h4>{props.title}</h4>
    {props.children}
  </section>
)

const Stat: Component<{
  label: string
  value: number | string
  sev?: 'err' | 'warn' | 'info'
  title?: string
}> = (props) => (
  <div class={`stat ${props.sev ? `stat-${props.sev}` : ''}`} title={props.title}>
    <div class="stat-val">{props.value}</div>
    <div class="stat-lbl">{props.label}</div>
  </div>
)

const EdgeRow: Component<{
  kind: EdgeKindLabel
  violation: string | null
  otherId: string
  dir: '→' | '←'
  data: DesignData
  onSelect: (id: string) => void
}> = (props) => {
  const other = () => props.data.moduleById.get(props.otherId)
  const layer = () => {
    const o = other()
    return o ? layerOfModule(props.data, o) : null
  }
  const vio = () => (props.violation ? props.data.violationById.get(props.violation) : undefined)
  return (
    <Show when={other()}>
      {(o) => (
        <div
          class={`edge-row ${vio() ? 'edge-row-vio' : ''}`}
          onClick={() => props.onSelect(props.otherId)}
        >
          <span class="edge-row-dir">{props.dir}</span>
          <span class="edge-row-kind" data-kind={props.kind}>
            {props.kind}
          </span>
          <span class="edge-row-name">
            <Show when={layer()}>
              {(l) => (
                <span class="edge-row-layer" style={{ color: layerColor(l().hue, TONE.edgeLayer) }}>
                  {l().label}
                </span>
              )}
            </Show>
            <span class="edge-row-mod"> / {o().label}</span>
          </span>
          <Show when={vio()}>
            {(v) => <span class={`edge-row-sev sev-${v().severity}`}>{v().severity}</span>}
          </Show>
        </div>
      )}
    </Show>
  )
}

const DetailsForModule: Component<{
  mod: DesignModule
  data: DesignData
  isPinned: boolean
  onClose: () => void
  cycle: CycleFocus
  onSelect: (id: string) => void
}> = (props) => {
  const containerLabel = () => props.data.containers.find((c) => c.id === props.mod.container)?.label
  const layer = () => layerOfModule(props.data, props.mod)
  const facet = () => facetInfo(props.mod.facet)
  const containerKind = () => containerKindOf(props.mod)
  const incoming = createMemo(() => props.data.edges.filter((e) => e.target === props.mod.id))
  const outgoing = createMemo(() => props.data.edges.filter((e) => e.source === props.mod.id))
  const vios = createMemo(() => violationsOf(props.data, props.mod))
  const kids = createMemo(() =>
    (props.data.childIndex[props.mod.id] ?? [])
      .map((id) => props.data.moduleById.get(id))
      .filter((m): m is DesignModule => m !== undefined),
  )
  const ancestors = createMemo(() =>
    ancestorIds(props.data.moduleById, props.mod.id)
      .map((id) => props.data.moduleById.get(id))
      .filter((m): m is DesignModule => m !== undefined)
      .reverse(),
  )

  return (
    <aside class="panel panel-details">
      <header class="panel-hd">
        <span class="panel-title">
          <Show when={layer()}>
            {(l) => (
              <span
                class="hd-pill"
                style={{
                  background: layerColor(l().hue, TONE.chipBg),
                  color: layerColor(l().hue, TONE.chipText),
                }}
              >
                {l().label}
              </span>
            )}
          </Show>
          <Show when={props.mod.kind === 'composer'}>
            <span class="hd-pill pill-kind-c">◇ composer</span>
          </Show>
          <Show when={props.mod.kind === 'shared'}>
            <span class="hd-pill pill-kind-s">_shared</span>
          </Show>
          <Show when={containerKind()}>
            {(k) => (
              <span class="hd-pill" data-testid="details-container-kind">
                {k()}
              </span>
            )}
          </Show>
          <span class="panel-title-mono">{props.mod.label}</span>
        </span>
        <Show when={props.isPinned}>
          <button class="close-x" onClick={props.onClose}>
            ×
          </button>
        </Show>
      </header>
      <div class="panel-body">
        <div class="breadcrumb">
          <Show when={containerLabel()}>
            {(label) => <span class="bc-item bc-ctr">{label()}</span>}
          </Show>
          <For each={ancestors()}>
            {(a) => (
              <>
                <span class="bc-sep">/</span>
                <span class="bc-item" onClick={() => props.onSelect(a.id)}>
                  {a.label}
                </span>
              </>
            )}
          </For>
          <span class="bc-sep">/</span>
          <span class="bc-item bc-self">{props.mod.label}</span>
        </div>
        <div class="det-path">{props.mod.path}</div>
        <div class="det-stats">
          <Stat
            label={`${facet().axis} · ${facet().name}`}
            value={props.mod.facet.value}
            title={facet().desc}
          />
          <Stat label="In" value={incoming().length} title="Incoming dependencies." />
          <Stat label="Out" value={outgoing().length} title="Outgoing dependencies." />
        </div>

        <Show when={kids().length > 0}>
          <Section title={`Children · ${kids().length}`}>
            <div class="kidgrid">
              <For each={kids()}>
                {(k) => {
                  const grandkids = () => props.data.childIndex[k.id]?.length ?? 0
                  return (
                    <div class={`kidgrid-item kind-${k.kind}`} onClick={() => props.onSelect(k.id)}>
                      <div class="kidgrid-name">
                        <Show when={k.kind === 'composer'}>
                          <span class="kidgrid-tag">◇</span>
                        </Show>
                        <Show when={k.kind === 'shared'}>
                          <span class="kidgrid-tag">_</span>
                        </Show>
                        {k.label}
                      </div>
                      <div class="kidgrid-meta">
                        {facetInfo(k.facet).axis.toLowerCase()} {k.facet.value}
                      </div>
                      <Show when={grandkids() > 0}>
                        <div class="kidgrid-deep">+{grandkids()} nested</div>
                      </Show>
                    </div>
                  )
                }}
              </For>
            </div>
          </Section>
        </Show>

        <Show when={vios().length > 0}>
          <Section title={`Violations · ${vios().length}`}>
            <div class="vios">
              <For each={vios()}>{(v) => <ViolationCard violation={v} cycle={props.cycle} />}</For>
            </div>
          </Section>
        </Show>

        <Section title={`Outgoing · ${outgoing().length}`}>
          <Show when={outgoing().length === 0}>
            <div class="empty">No outgoing imports.</div>
          </Show>
          <For each={outgoing()}>
            {(e) => (
              <EdgeRow
                kind={e.kind}
                violation={e.violation}
                otherId={e.target}
                dir="→"
                data={props.data}
                onSelect={props.onSelect}
              />
            )}
          </For>
        </Section>

        <Section title={`Incoming · ${incoming().length}`}>
          <Show when={incoming().length === 0}>
            <div class="empty">No dependents.</div>
          </Show>
          <For each={incoming()}>
            {(e) => (
              <EdgeRow
                kind={e.kind}
                violation={e.violation}
                otherId={e.source}
                dir="←"
                data={props.data}
                onSelect={props.onSelect}
              />
            )}
          </For>
        </Section>
      </div>
    </aside>
  )
}

const DetailsEmpty: Component<{
  data: DesignData
  cycle: CycleFocus
  isViolationsUnavailable: boolean
}> = (props) => {
  const totals = createMemo(() => {
    const count = (s: DesignViolation['severity']) =>
      props.data.violations.filter((v) => v.severity === s).length
    return { err: count('error'), warn: count('warning'), info: count('info') }
  })
  return (
    <aside class="panel panel-details">
      <header class="panel-hd">
        <span class="panel-title">
          <span class="panel-title-mono">{props.data.meta.project}</span>
        </span>
      </header>
      <div class="panel-body">
        <div class="snapshot-row">
          snapshot v{props.data.meta.snapshotVersion} · loaded {props.data.meta.loadedAt.slice(0, 10)}
        </div>
        <Show when={props.isViolationsUnavailable}>
          <div class="empty" role="alert">
            Violations unavailable: the server did not return them.
          </div>
        </Show>
        <div class="det-stats det-stats-grid">
          <Stat label="Modules" value={props.data.modules.length} />
          <Stat label="Containers" value={props.data.containers.length} />
          <Stat label="Edges" value={props.data.edges.length} />
          <Stat label="Errors" value={totals().err} sev="err" />
          <Stat label="Warnings" value={totals().warn} sev="warn" />
          <Stat label="Info" value={totals().info} sev="info" />
        </div>
        <Section title={`All violations · ${props.data.violations.length}`}>
          <div class="vios">
            <For each={props.data.violations}>
              {(v) => <ViolationCard violation={v} cycle={props.cycle} />}
            </For>
          </div>
        </Section>
        <Section title="Tip">
          <div class="empty">
            <div>Hover a module to inspect.</div>
            <div>Drag empty canvas to pan, scroll to zoom.</div>
            <div>
              <kbd>⌘P</kbd> · <kbd>/</kbd> command palette · <kbd>Esc</kbd> reset
            </div>
          </div>
        </Section>
      </div>
    </aside>
  )
}

export const DetailsPanel: Component<{
  data: DesignData
  selectedId: string | null
  hoveredId: string | null
  onClose: () => void
  onFocusCycle: (vid: string) => void
  focusedCycle: string | null
  onSelect: (id: string) => void
  isViolationsUnavailable: boolean
}> = (props) => {
  const mod = () => {
    const id = props.selectedId ?? props.hoveredId
    return id ? props.data.moduleById.get(id) : undefined
  }
  const cycle: CycleFocus = {
    get focusedCycle() {
      return props.focusedCycle
    },
    onFocusCycle: (vid) => props.onFocusCycle(vid),
  }

  return (
    <Show
      when={mod()}
      fallback={
        <DetailsEmpty
          data={props.data}
          cycle={cycle}
          isViolationsUnavailable={props.isViolationsUnavailable}
        />
      }
    >
      {(m) => (
        <DetailsForModule
          mod={m()}
          data={props.data}
          isPinned={props.selectedId !== null}
          onClose={props.onClose}
          cycle={cycle}
          onSelect={props.onSelect}
        />
      )}
    </Show>
  )
}
