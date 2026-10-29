import { Component, createResource, For, Show } from 'solid-js'
import { exportSvg } from '../../canvas/canvas'
import { ContractSection } from '../../contract/contract-section'
import { t } from '../../i18n'
import { SEV_COLOR } from '../../render/encoding'
import { Model, worst } from '../../render/model'
import { go, nav } from '../../shell/nav'
import { PathText } from '../../shell/path-text'
import { Violation } from '../../types'
import { withMember } from '../set/analysis'
import { ViewProps } from '../view-def'
import { resolveFocus } from './focus-target'
import { layerName, layerTone, shortRule } from './paint'

const MAX_VIOLATIONS = 40
const MAX_PAIRS = 30
const SVG_NAME_UNSAFE = /\W+/g

const fetchBrief = async (path: string): Promise<string | null> => {
  const resp = await fetch(`/api/context?path=${encodeURIComponent(path)}`)
  return resp.ok ? resp.text() : null
}

const Brief: Component<{ path: string }> = (props) => {
  const [brief] = createResource(() => props.path, fetchBrief)
  const text = () => (brief.state === 'ready' ? brief() : null)
  return (
    <Show when={text()}>
      {(body) => (
        <div class="det-section">
          <h4>{t('focus.details.brief')}</h4>
          <pre class="fo-brief" data-testid="focus-brief">
            <PathText path={body()} />
          </pre>
        </div>
      )}
    </Show>
  )
}

const SevDot: Component<{ findings: readonly Violation[] }> = (props) => {
  const sev = () => worst(props.findings)
  return (
    <Show when={sev()}>
      <span class="fo-dot" data-ch="sev" style={{ background: SEV_COLOR[sev()!] }} />
    </Show>
  )
}

const ViolationList: Component<{ findings: readonly Violation[] }> = (props) => (
  <Show when={props.findings.length > 0} fallback={<div class="empty">{t('focus.details.none')}</div>}>
    <div class="vios">
      <For each={props.findings.slice(0, MAX_VIOLATIONS)}>
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
      <Show when={props.findings.length > MAX_VIOLATIONS}>
        <div class="empty">{t('focus.details.more', { n: props.findings.length - MAX_VIOLATIONS })}</div>
      </Show>
    </div>
  </Show>
)

const LayerBars: Component<{ model: Model; ids: readonly number[] }> = (props) => {
  const rows = () => {
    const counts = new Map<number, number>()
    for (const id of props.ids) {
      const layer = props.model.modules.get(id)?.layer
      if (layer !== undefined) counts.set(layer, (counts.get(layer) ?? 0) + 1)
    }
    return [...counts.entries()].sort((a, b) => a[0] - b[0])
  }
  const max = () => Math.max(1, ...rows().map(([, n]) => n))
  return (
    <Show when={rows().length > 0} fallback={<div class="empty">{t('focus.details.none')}</div>}>
      <div class="fo-bars">
        <For each={rows()}>
          {([layer, n]) => (
            <div class="fo-bar">
              <span>{layerName(props.model, layer)}</span>
              <div class="fo-bar-t">
                <div class="fo-bar-f" style={{ width: `${(n / max()) * 100}%`, background: layerTone(props.model, layer) }} />
              </div>
              <span class="fo-n">{n}</span>
            </div>
          )}
        </For>
      </div>
    </Show>
  )
}

const ImportsDetail: Component<{ model: Model; focus: number; pairs: readonly (readonly [number, number])[] }> = (
  props,
) => {
  const m = props.model
  const findings = () => props.pairs.flatMap(([a, b]) => m.findingsOnEdge(a, b))
  const other = (a: number, b: number) => (a === props.focus ? b : a)
  return (
    <div class="det-section" data-testid="focus-imports">
      <h4>{t('focus.details.bundle', { n: props.pairs.length })}</h4>
      <div class="fo-rows">
        <For each={props.pairs.slice(0, MAX_PAIRS)}>
          {([a, b]) => {
            const onEdge = m.findingsOnEdge(a, b)
            return (
              <div
                class="fo-row-item"
                title={`${m.modules.get(a)?.path}\n→ ${m.modules.get(b)?.path}`}
                onClick={() => go({ view: 'focus', focus: other(a, b) })}
              >
                <span class="fo-nm">
                  {m.label(a)} → {m.label(b)}
                </span>
                <Show when={onEdge.length > 0}>
                  <span class="fo-n" data-rule={onEdge.map((v) => v.rule).join(' ')}>
                    {[...new Set(onEdge.map((v) => shortRule(v.rule)))].join(', ')}
                  </span>
                </Show>
                <SevDot findings={onEdge} />
              </div>
            )
          }}
        </For>
        <Show when={props.pairs.length > MAX_PAIRS}>
          <div class="empty">{t('focus.details.more', { n: props.pairs.length - MAX_PAIRS })}</div>
        </Show>
      </div>
      <Show when={findings().length > 0}>
        <ViolationList findings={findings()} />
      </Show>
    </div>
  )
}

const importsSelection = () => {
  const sel = nav().sel
  return sel?.kind === 'imports' ? sel : null
}

export const FocusDetails: Component<ViewProps> = (props) => {
  const focus = () => resolveFocus(props.model, nav().focus)
  const module = () => props.model.modules.get(focus()!)!
  return (
    <aside class="panel panel-details">
      <header class="panel-hd">
        <span class="panel-title">{t('focus.details.title')}</span>
        <Show when={focus() !== null}>
          <span class="panel-count">{layerName(props.model, module().layer)}</span>
        </Show>
      </header>
      <Show when={focus() !== null}>
        <div class="panel-body">
          <Show when={importsSelection()}>
            {(s) => (
              <>
                <ImportsDetail model={props.model} focus={focus()!} pairs={s().pairs} />
                <ContractSection edges={s().pairs.map(([from, to]) => ({ level: 'module', from, to }))} />
              </>
            )}
          </Show>
          <div class="fo-det-title">{props.model.label(focus()!)}</div>
          <div class="det-path">
            <PathText path={module().path} />
          </div>
          <div class="fo-btn-row">
            <button
              class="fo-btn"
              onClick={() => exportSvg(`focus-${props.model.label(focus()!).replace(SVG_NAME_UNSAFE, '-')}`)}
            >
              {t('focus.details.svg')}
            </button>
            <button class="fo-btn" data-testid="focus-add-to-set" onClick={() => go({ view: 'set', set: withMember(nav().set, focus()!) })}>
              {t('set.addToSet')}
            </button>
          </div>
          <Brief path={module().path} />
          <div class="det-section">
            <h4>{t('focus.details.violations')}</h4>
            <ViolationList findings={props.model.findingsOf(focus()!)} />
          </div>
          <div class="det-section">
            <h4>{t('focus.details.imports', { n: props.model.out(focus()!).length })}</h4>
            <LayerBars model={props.model} ids={props.model.out(focus()!)} />
          </div>
          <div class="det-section">
            <h4>{t('focus.details.importedBy', { n: props.model.inn(focus()!).length })}</h4>
            <LayerBars model={props.model} ids={props.model.inn(focus()!)} />
          </div>
        </div>
      </Show>
    </aside>
  )
}

