import { Component, createMemo, For, Show } from 'solid-js'
import { t } from '../../i18n'
import { Model } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { PathText } from '../../shell/path-text'
import { DriftReport } from './drift'
import { describeItems } from './items'
import { rulesOf, shortPath } from './paint'
import { Review } from './review'

const LIST_LIMIT = 60

const selectedIs = (from: number, to: number): boolean => {
  const sel = nav().sel
  return sel?.kind === 'edge' && sel.from === from && sel.to === to
}

export const MrOutline: Component<{ review: Review }> = (props) => (
  <div class="panel">
    <div class="panel-hd">
      <span class="panel-title">{t('mr.files')}</span>
      <span class="panel-count">{props.review.files.length}</span>
    </div>
    <div class="outline-tree mr-files">
      <For each={props.review.files}>
        {(f) => (
          <button
            class="mr-file"
            data-testid="mr-file"
            title={f.path}
            disabled={f.isGhost}
            onClick={() => go({ view: 'focus', focus: f.id })}
          >
            <span class="mr-dot" classList={{ [`sev-${f.worst}`]: f.worst !== null }} data-ch={f.worst ? 'sev' : undefined} />
            <span class="nm mono">{shortPath(f.path)}</span>
            <span class="n mono">{f.isGhost ? t('mr.file.gone') : `+${f.added} −${f.removed}`}</span>
          </button>
        )}
      </For>
      <p class="mr-note">{t('mr.files.note')}</p>
    </div>
  </div>
)

const SelectedEdge: Component<{ review: Review }> = (props) => {
  const edge = createMemo(() => {
    const sel = nav().sel
    if (sel?.kind !== 'edge') return null
    return props.review.edges.find((e) => e.from === sel.from && e.to === sel.to) ?? null
  })
  return (
    <Show when={edge()}>
      {(e) => (
        <div class="det-section" data-testid="mr-selected">
          <h4>{t(`mr.tag.${e().tag}`)}</h4>
          <div class="det-path mono">
            <PathText path={props.review.nodes.get(e().from)?.path ?? ''} />
            <br />→ <PathText path={props.review.nodes.get(e().to)?.path ?? ''} />
          </div>
          <Show when={e().breaks.length}>
            <div class="mono mr-rules">{rulesOf(e().breaks)}</div>
          </Show>
        </div>
      )}
    </Show>
  )
}

export const MrDetails: Component<{ model: Model; review: Review; drift: DriftReport }> = (props) => {
  const items = createMemo(() => describeItems(props.model, props.review.items.slice(0, LIST_LIMIT)))
  const c = () => props.review.counts
  return (
    <div class="panel panel-details">
      <div class="panel-hd">
        <span class="panel-title">{t('mr.title')}</span>
        <span class="panel-count mono">{t('mr.vs', { ref: props.drift.base.ref, sha: props.drift.base.sha })}</span>
      </div>
      <div class="panel-body">
        <SelectedEdge review={props.review} />
        <div class="det-stats det-stats-grid" data-testid="mr-stats">
          <div class="stat stat-err">
            <div class="stat-val" data-testid="mr-breaking" data-ch="sev">
              {c().breaking}
            </div>
            <div class="stat-lbl">{t('mr.stat.breaking', { n: c().breaking })}</div>
          </div>
          <div class="stat">
            <div class="stat-val">
              +{c().added} −{c().removed}
            </div>
            <div class="stat-lbl">{t('mr.stat.imports')}</div>
          </div>
          <div class="stat">
            <div class="stat-val">{c().cycles + c().stages}</div>
            <div class="stat-lbl">
              {t('mr.stat.cycles', { n: c().cycles })} / {t('mr.stat.stages', { n: c().stages })}
            </div>
          </div>
        </div>
        <Show when={props.review.omitted}>
          <p class="mr-note" data-testid="mr-omitted">
            {t('mr.omitted', { n: props.review.omitted })}
          </p>
        </Show>
        <div class="det-section">
          <h4>{t('mr.changes', { n: props.review.items.length })}</h4>
          <div class="mr-items">
            <For each={items()}>
              {(it) => (
                <button
                  class="mr-item"
                  data-testid="mr-item"
                  classList={{ on: it.edge !== null && selectedIs(it.edge.from, it.edge.to) }}
                  onClick={() => {
                    if (it.edge) set({ sel: { kind: 'edge', from: it.edge.from, to: it.edge.to } })
                  }}
                >
                  <span class="mr-item-hd">
                    <span
                      class="mr-chg"
                      classList={{ fixed: it.isFixed, [`sev-${it.severity}`]: it.severity !== null }}
                      data-ch={it.severity ? 'sev' : undefined}
                    >
                      {it.pill}
                    </span>
                    <span class="mono mr-item-sub">{it.sub}</span>
                  </span>
                  <span class="mono mr-item-main">
                    <PathText path={it.main} />
                  </span>
                </button>
              )}
            </For>
          </div>
        </div>
      </div>
    </div>
  )
}
