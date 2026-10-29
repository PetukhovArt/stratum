import { Component, createMemo, createResource, For, Match, Show, Switch } from 'solid-js'
import { t } from '../i18n'
import { ApiStatusError, fetchSource, Loc } from '../lib/api'
import { PathLabel } from '../shell/path-text'
import { closeCode, codeTarget, CodeTarget } from './code'
import { rowsOf } from './source'
import './contract.css'

const CONTEXT = { decl: 12, import: 4 } as const

const loadHighlighter = () => import('./highlight')

const reasonOf = (err: unknown): string => {
  if (err instanceof ApiStatusError) return String(err.status)
  return err instanceof Error ? err.message : String(err)
}

const centerInBox = (row: HTMLElement): void => {
  queueMicrotask(() => {
    const box = row.parentElement
    if (box) box.scrollTop = row.offsetTop - box.clientHeight / 2
  })
}

const Snippet: Component<{ role: 'decl' | 'import'; at: Loc }> = (props) => {
  const [source] = createResource(() => ({ ...props.at, context: CONTEXT[props.role] }), fetchSource)
  const [highlighter] = createResource(loadHighlighter)
  const rows = createMemo(() => {
    if (source.state !== 'ready') return []
    const s = source()
    const html = highlighter.state === 'ready' ? highlighter().highlightedLinesOf(s.path, s.lines) : null
    return rowsOf(s, props.at.line, html)
  })
  return (
    <div class="code-snippet" data-testid="code-snippet" data-role={props.role} data-at={`${props.at.path}:${props.at.line}`}>
      <div class="code-snippet-hd" title={`${props.at.path}:${props.at.line}`}>
        <span class="code-role">{t(props.role === 'decl' ? 'code.decl' : 'code.import')}</span>
        <PathLabel path={`${props.at.path}:${props.at.line}`} />
      </div>
      <Switch>
        <Match when={source.state === 'errored'}>
          <div class="empty code-msg">{t('code.missing', { reason: reasonOf(source.error) })}</div>
        </Match>
        <Match when={source.state === 'ready'}>
          <div class="code-lines">
            <For each={rows()}>
              {(row) => (
                <div
                  class={`code-line${row.isMarked ? ' on' : ''}`}
                  data-testid="code-line"
                  data-line={row.line}
                  data-marked={row.isMarked ? 'true' : undefined}
                  ref={(el) => row.isMarked && centerInBox(el)}
                >
                  <span class="code-n">{row.line}</span>
                  <Show when={row.html} fallback={<span class="code-t">{row.text}</span>}>
                    {(html) => <span class="code-t" innerHTML={html()} />}
                  </Show>
                </div>
              )}
            </For>
          </div>
        </Match>
        <Match when={source.loading}>
          <div class="empty code-msg">{t('code.loading')}</div>
        </Match>
      </Switch>
    </div>
  )
}

const CodeDock: Component<{ target: CodeTarget }> = (props) => (
  <section class="code-dock" data-testid="code-panel" aria-label={t('code.title')}>
    <header class="code-hd">
      <span class="code-title">{t('code.title')}</span>
      <span class="code-name mono" data-testid="code-name">
        {props.target.name}
      </span>
      <Show when={!props.target.decl}>
        <span class="code-note">{t('code.unresolved')}</span>
      </Show>
      <span class="sp" />
      <kbd>Esc</kbd>
      <button class="code-x" data-testid="code-close" aria-label={t('code.close')} title={t('code.close')} onClick={closeCode}>
        ×
      </button>
    </header>
    <div class="code-body">
      <Show when={props.target.site}>{(site) => <Snippet role="import" at={site()} />}</Show>
      <Show when={props.target.decl}>{(decl) => <Snippet role="decl" at={decl()} />}</Show>
    </div>
  </section>
)

export const CodePanel: Component = () => <Show when={codeTarget()}>{(target) => <CodeDock target={target()} />}</Show>
