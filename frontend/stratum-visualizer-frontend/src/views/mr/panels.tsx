import { Component, createMemo, For, Show } from 'solid-js'
import { t } from '../../i18n'
import { Model } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { PathText } from '../../shell/path-text'
import { DriftReport } from './drift'
import { describeItems } from './items'
import { labelOf, rulesOf, shortPath } from './paint'
import { edgeOf, Review, ReviewEdge, worstBreak } from './review'

const LIST_LIMIT = 60

const selectedIs = (from: number, to: number): boolean => {
  const sel = nav().sel
  return sel?.kind === 'edge' && sel.from === from && sel.to === to
}

const selectedModule = (): number | null => {
  const sel = nav().sel
  return sel?.kind === 'module' ? sel.id : null
}

const openFocus = (id: number): void => {
  if (id >= 0) go({ view: 'focus', focus: id })
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
            classList={{ gone: f.isGhost, on: selectedModule() === f.id }}
            data-testid="mr-file"
            title={f.path}
            onClick={() => set({ sel: { kind: 'module', id: f.id } })}
            onDblClick={() => openFocus(f.id)}
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

const ImportRows: Component<{ model: Model; review: Review; edges: readonly ReviewEdge[] }> = (props) => (
  <div class="mr-items" data-testid="mr-selected-imports">
    <For each={props.edges.slice(0, LIST_LIMIT)}>
      {(e) => {
        const sev = worstBreak(e.breaks)
        const label = (id: number) => labelOf(props.model, id, props.review.nodes.get(id)?.path ?? '')
        return (
          <button
            class="mr-item"
            data-testid="mr-selected-import"
            onClick={() => set({ sel: { kind: 'edge', from: e.from, to: e.to } })}
          >
            <span class="mr-item-hd">
              <span
                class="mr-chg"
                classList={{ fixed: e.tag === 'removed', [`sev-${sev}`]: sev !== null }}
                data-ch={sev ? 'sev' : undefined}
              >
                {t(`mr.tag.${e.tag}`)}
              </span>
              <span class="mono mr-item-sub">{rulesOf(e.breaks)}</span>
            </span>
            <span class="mono mr-item-main">
              {label(e.from)} {e.tag === 'removed' ? '↛' : '→'} {label(e.to)}
            </span>
          </button>
        )
      }}
    </For>
  </div>
)

const SelectedEdge: Component<{ model: Model; review: Review }> = (props) => {
  const edge = createMemo(() => {
    const sel = nav().sel
    return sel?.kind === 'edge' ? edgeOf(props.model, props.review, sel.from, sel.to) : null
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

const SelectedModule: Component<{ model: Model; review: Review }> = (props) => {
  const node = createMemo(() => {
    const id = selectedModule()
    return id === null ? undefined : props.review.nodes.get(id)
  })
  const changes = createMemo(() => {
    const n = node()
    return n ? props.review.changes.filter((e) => e.from === n.id || e.to === n.id) : []
  })
  return (
    <Show when={node()}>
      {(n) => (
        <div class="det-section" data-testid="mr-selected">
          <h4>{t('mr.sel.module', { n: changes().length })}</h4>
          <div class="det-path mono">
            <PathText path={n().path} />
          </div>
          <Show when={!n().isGhost}>
            <button class="mr-btn" data-testid="mr-open-focus" onClick={() => openFocus(n().id)}>
              {t('mr.sel.open')}
            </button>
          </Show>
          <ImportRows model={props.model} review={props.review} edges={changes()} />
        </div>
      )}
    </Show>
  )
}

const SelectedFold: Component<{ model: Model; review: Review }> = (props) => {
  const fold = createMemo(() => {
    const sel = nav().sel
    return sel?.kind === 'link' ? props.review.folds.find((f) => f.from === sel.from && f.to === sel.to) : undefined
  })
  const pathOf = (key: string) => props.review.folderOf.get(key)?.path ?? ''
  return (
    <Show when={fold()}>
      {(f) => (
        <div class="det-section" data-testid="mr-selected">
          <h4>{t('mr.sel.fold', { added: f().added, removed: f().removed })}</h4>
          <div class="det-path mono">
            <PathText path={pathOf(f().from)} />
            <br />→ <PathText path={pathOf(f().to)} />
          </div>
          <ImportRows model={props.model} review={props.review} edges={f().edges} />
        </div>
      )}
    </Show>
  )
}

const SelectedFolder: Component<{ model: Model; review: Review }> = (props) => {
  const folder = createMemo(() => {
    const sel = nav().sel
    return sel?.kind === 'box' ? props.review.folderOf.get(sel.key) : undefined
  })
  const edges = createMemo(() => {
    const f = folder()
    if (!f) return []
    const inside = (id: number) => props.review.nodes.get(id)?.group.key === f.key
    const rank = (e: ReviewEdge) => (e.breaks.length ? 0 : 1)
    return props.review.changes.filter((e) => inside(e.from) || inside(e.to)).sort((a, b) => rank(a) - rank(b))
  })
  return (
    <Show when={folder()}>
      {(f) => (
        <div class="det-section" data-testid="mr-selected">
          <h4>{t('mr.sel.folder', { n: edges().length })}</h4>
          <div class="det-path mono">
            <PathText path={f().path} />
          </div>
          <ImportRows model={props.model} review={props.review} edges={edges()} />
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
        <SelectedEdge model={props.model} review={props.review} />
        <SelectedModule model={props.model} review={props.review} />
        <SelectedFold model={props.model} review={props.review} />
        <SelectedFolder model={props.model} review={props.review} />
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
