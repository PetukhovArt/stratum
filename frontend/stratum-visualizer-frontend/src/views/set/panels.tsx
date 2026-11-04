import { Component, createMemo, createSignal, For, Show } from 'solid-js'
import { exportSvg } from '../../canvas/canvas'
import { ContractSection } from '../../contract/contract-section'
import { t } from '../../i18n'
import { SEV_COLOR } from '../../render/encoding'
import { edgeKey, Model, worst } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { PathLabel, PathText } from '../../shell/path-text'
import { Violation } from '../../types'
import { layerTone, shortRule } from '../focus/paint'
import { MAX_PATH_HOPS, moduleCycles, SET_LIMIT, SetAnalysis, withMember, withoutMember } from './analysis'

const MAX_CUTS = 6
const MAX_PATHS = 20
const MAX_FINDINGS = 20
const MAX_MATCHES = 40

const SevDot: Component<{ findings: readonly Violation[] }> = (props) => (
  <Show when={worst(props.findings)}>
    {(s) => <span class="set-dot" data-ch="sev" style={{ background: SEV_COLOR[s()] }} />}
  </Show>
)

const Swatch: Component<{ model: Model; id: number }> = (props) => (
  <span class="set-sw" style={{ background: layerTone(props.model, props.model.modules.get(props.id)!.layer) }} />
)

const More: Component<{ total: number; shown: number }> = (props) => (
  <Show when={props.total > props.shown}>
    <div class="empty">{t('set.details.more', { n: props.total - props.shown })}</div>
  </Show>
)

const SelectedImport: Component<{ model: Model; from: number; to: number }> = (props) => {
  const m = props.model
  const findings = () => m.findingsOnEdge(props.from, props.to)
  return (
    <div class="det-section" data-testid="set-import">
      <h4>{t('set.details.import')}</h4>
      <div class="det-path">
        <PathText path={m.modules.get(props.from)?.path ?? ''} />
        <br />→ <PathText path={m.modules.get(props.to)?.path ?? ''} />
      </div>
      <Show when={findings().length} fallback={<div class="empty">{t('set.details.noFinding')}</div>}>
        <div class="vios">
          <For each={findings().slice(0, MAX_FINDINGS)}>
            {(v) => (
              <div class={`vio vio-${v.severity}`} data-ch="sev">
                <div class="vio-hd">
                  <span class={`vio-sev sev-${v.severity}`}>{v.severity}</span>
                  <span class="vio-rule">{shortRule(v.rule)}</span>
                </div>
                <div class="vio-msg">{v.message}</div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  )
}

const ModuleRow: Component<{ model: Model; id: number; trailing?: string; onRemove?: () => void }> = (props) => (
  <div class="set-row" title={props.model.modules.get(props.id)?.path} onClick={() => go({ view: 'focus', focus: props.id })}>
    <Swatch model={props.model} id={props.id} />
    <span class="set-nm">{props.model.label(props.id)}</span>
    <Show when={props.trailing}>
      <span class="set-n">{props.trailing}</span>
    </Show>
    <SevDot findings={props.model.findingsOf(props.id)} />
    <Show when={props.onRemove}>
      {(remove) => (
        <button
          class="set-x"
          title={t('set.details.remove')}
          onClick={(e) => {
            e.stopPropagation()
            remove()()
          }}
        >
          ×
        </button>
      )}
    </Show>
  </div>
)

export const SetDetails: Component<{ model: Model; analysis: SetAnalysis }> = (props) => {
  const m = props.model
  const a = () => props.analysis
  const inSet = createMemo(() => new Set(a().members))
  const insideCount = (id: number) => {
    const into = m.inn(id).filter((x) => inSet().has(x)).length
    const from = m.out(id).filter((x) => inSet().has(x)).length
    return `${into}← ${from}→`
  }
  const induced = () => a().edges.filter((e) => !e.isPath).length
  const edgeSel = () => {
    const s = nav().sel
    return s?.kind === 'edge' ? s : null
  }
  return (
    <aside class="panel panel-details">
      <header class="panel-hd">
        <span class="panel-title">{t('set.details.title')}</span>
        <span class="panel-count">{a().members.length}</span>
      </header>
      <div class="panel-body">
        <Show when={edgeSel()}>
          {(s) => (
            <>
              <SelectedImport model={m} from={s().from} to={s().to} />
              <ContractSection edges={[{ level: 'module', from: s().from, to: s().to }]} />
            </>
          )}
        </Show>
        <div class="det-stats det-stats-grid" data-testid="set-stats">
          <div class="stat">
            <div class="stat-val">{a().members.length}</div>
            <div class="stat-lbl">{t('set.stat.members', { n: a().members.length })}</div>
          </div>
          <div class="stat">
            <div class="stat-val">{induced()}</div>
            <div class="stat-lbl">{t('set.stat.inside', { n: induced() })}</div>
          </div>
          <div class="stat">
            <div class="stat-val" data-testid="set-loop-size">
              {a().loop?.length ?? 0}
            </div>
            <div class="stat-lbl">{t('set.stat.loop')}</div>
          </div>
        </div>
        <Show when={a().chain}>
          {(chain) => (
            <div class="det-section" data-testid="set-chain">
              <h4>{t('set.details.loop')}</h4>
              <div class="set-rows">
                <For each={chain()}>
                  {(id, i) => <ModuleRow model={m} id={id} trailing={i() === chain().length - 1 ? '↺' : String(i() + 1)} />}
                </For>
              </div>
              <p class="set-note">{t('set.details.loopNote')}</p>
            </div>
          )}
        </Show>
        <Show when={a().cuts.length}>
          <div class="det-section" data-testid="set-cuts">
            <h4>{t('set.details.cuts')}</h4>
            <div class="set-rows">
              <For each={a().cuts.slice(0, MAX_CUTS)}>
                {(c) => (
                  <div
                    class={`set-row${edgeSel() && edgeKey(edgeSel()!.from, edgeSel()!.to) === edgeKey(c.from, c.to) ? ' on' : ''}`}
                    data-testid="set-cut-row"
                    data-edge={edgeKey(c.from, c.to)}
                    data-freed={c.freed}
                    onClick={() => set({ sel: { kind: 'edge', from: c.from, to: c.to } })}
                  >
                    <span class="set-nm set-pair">
                      <PathLabel path={m.label(c.from)} />
                      <span class="set-pair-to">
                        → <PathLabel path={m.label(c.to)} />
                      </span>
                    </span>
                    <span class="set-n">
                      {t('set.details.frees', { n: c.freed })}
                    </span>
                    <SevDot findings={m.findingsOnEdge(c.from, c.to)} />
                  </div>
                )}
              </For>
              <More total={a().cuts.length} shown={MAX_CUTS} />
            </div>
            <p class="set-note">{t('set.details.cutsNote')}</p>
          </div>
        </Show>
        <div class="det-section" data-testid="set-members">
          <h4>{t('set.details.members', { n: a().members.length })}</h4>
          <div class="set-rows">
            <For each={a().members}>
              {(id) => (
                <ModuleRow
                  model={m}
                  id={id}
                  trailing={insideCount(id)}
                  onRemove={() => go({ set: withoutMember(nav().set, id) })}
                />
              )}
            </For>
          </div>
        </div>
        <div class="det-section" data-testid="set-paths">
          <h4>{t('set.details.paths', { n: a().paths.length })}</h4>
          <Show when={a().paths.length} fallback={<div class="empty">{t('set.details.noPaths', { n: MAX_PATH_HOPS })}</div>}>
            <div class="set-rows">
              <For each={a().paths.slice(0, MAX_PATHS)}>
                {(p) => (
                  <div class="set-path mono" title={p.map((id) => m.modules.get(id)?.path).join('\n→ ')}>
                    <For each={p}>
                      {(id, i) => (
                        <>
                          <Show when={i() > 0}>
                            <span class="set-arrow">→</span>
                          </Show>
                          <span class={inSet().has(id) ? 'set-p-member' : 'set-p-via'} onClick={() => go({ view: 'focus', focus: id })}>
                            {m.label(id)}
                          </span>
                        </>
                      )}
                    </For>
                  </div>
                )}
              </For>
              <More total={a().paths.length} shown={MAX_PATHS} />
            </div>
          </Show>
        </div>
        <div class="set-btn-row">
          <button class="set-btn" onClick={() => exportSvg(`set-${a().members.length}`)}>
            {t('set.details.svg')}
          </button>
          <button class="set-btn" onClick={() => go({ set: [] })}>
            {t('set.details.clear')}
          </button>
        </div>
      </div>
    </aside>
  )
}

const sameMembers = (a: readonly number[], b: readonly number[]): boolean => {
  if (a.length !== b.length) return false
  const s = new Set(a)
  return b.every((x) => s.has(x))
}

export const SetOutline: Component<{ model: Model }> = (props) => {
  const m = props.model
  const [query, setQuery] = createSignal('')
  const cycles = createMemo(() => moduleCycles(m))
  const presets = () => cycles().filter((c) => c.length <= SET_LIMIT)
  const tooLarge = () => cycles().length - presets().length
  const matches = createMemo(() => {
    const q = query().trim().toLowerCase()
    if (!q) return []
    return [...m.modules.values()]
      .filter((mod) => mod.path.toLowerCase().includes(q))
      .slice(0, MAX_MATCHES)
      .map((mod) => mod.id)
  })
  const toggle = (id: number) => {
    const current = nav().set
    go({ set: current.includes(id) ? withoutMember(current, id) : withMember(current, id) })
  }
  return (
    <aside class="panel">
      <header class="panel-hd">
        <span class="panel-title">{t('set.outline.title')}</span>
        <span class="panel-count">{nav().set.length}</span>
      </header>
      <input
        class="set-filter"
        placeholder={t('set.outline.filter')}
        value={query()}
        onInput={(e) => setQuery(e.currentTarget.value)}
      />
      <div class="set-ol">
        <For each={matches()}>
          {(id) => (
            <div
              class={`set-ol-row${nav().set.includes(id) ? ' on' : ''}`}
              data-testid="set-match"
              title={m.modules.get(id)?.path}
              onClick={() => toggle(id)}
            >
              <Swatch model={m} id={id} />
              <span class="set-nm">{m.modules.get(id)?.path}</span>
            </div>
          )}
        </For>
        <div class="set-ol-sec">{t('set.outline.cycles')}</div>
        <Show when={cycles().length} fallback={<div class="set-ol-note" data-testid="set-no-cycles">{t('set.outline.noCycles')}</div>}>
          <For each={presets()}>
            {(c) => (
              <div
                class={`set-preset${sameMembers(c, nav().set) ? ' on' : ''}`}
                data-testid="set-preset"
                onClick={() => go({ view: 'set', set: [...c] })}
              >
                <b>{t('set.outline.cycle', { n: c.length })}</b>
                <span class="mono">{c.slice(0, 3).map(m.label).join(', ')}{c.length > 3 ? '…' : ''}</span>
              </div>
            )}
          </For>
          <Show when={tooLarge()}>
            <div class="set-ol-note">{t('set.outline.tooLarge', { n: tooLarge(), limit: SET_LIMIT })}</div>
          </Show>
        </Show>
      </div>
    </aside>
  )
}
