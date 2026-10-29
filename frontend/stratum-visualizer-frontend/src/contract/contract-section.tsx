import { Component, createResource, For, Match, Show, Switch } from 'solid-js'
import { t } from '../i18n'
import { EdgeEnds, fetchEdge, Loc } from '../lib/api'
import { PathLabel } from '../shell/path-text'
import { codeTarget, CodeTarget, openCode } from './code'
import { Contract, contractOf, UsedName } from './contract'
import './contract.css'

const MAX_EDGES = 30
const MAX_NAMES = 60
const MAX_FILES = 30

const locText = (at: Loc): string => `${at.path}:${at.line}`

const sameLoc = (a: Loc | null, b: Loc | null): boolean => a?.path === b?.path && a?.line === b?.line

const isOpen = (target: CodeTarget): boolean => {
  const open = codeTarget()
  return !!open && open.name === target.name && sameLoc(open.decl, target.decl) && sameLoc(open.site, target.site)
}

const loadContract = async (edges: readonly EdgeEnds[]): Promise<Contract> =>
  contractOf((await Promise.all(edges.slice(0, MAX_EDGES).map(fetchEdge))).flat())

const TypeBadge: Component = () => (
  <span class="ct-type" data-testid="contract-type-only" title={t('contract.typeOnlyTitle')}>
    {t('contract.typeOnly')}
  </span>
)

const More: Component<{ total: number; shown: number }> = (props) => (
  <Show when={props.total > props.shown}>
    <div class="empty">{t('contract.more', { n: props.total - props.shown })}</div>
  </Show>
)

const NameRow: Component<{ name: UsedName }> = (props) => {
  const target = (): CodeTarget => ({
    name: props.name.name,
    signature: null,
    decl: props.name.decl,
    site: { ...props.name.site, kind: 'import' },
  })
  return (
    <button
      class={`ct-name${isOpen(target()) ? ' on' : ''}`}
      data-testid="contract-name"
      data-name={props.name.name}
      data-decl={props.name.decl ? locText(props.name.decl) : undefined}
      title={props.name.decl ? locText(props.name.decl) : t('contract.unresolved')}
      onClick={() => openCode(target())}
    >
      <span class="ct-nm">{props.name.name}</span>
      <Show when={props.name.isTypeOnly}>
        <TypeBadge />
      </Show>
      <span class="ct-decl">
        <Show when={props.name.decl} fallback={<span class="ct-unresolved">{t('contract.unresolved')}</span>}>
          {(decl) => <PathLabel path={locText(decl())} />}
        </Show>
      </span>
      <Show when={props.name.importers > 1}>
        <span class="ct-n">{t('contract.importers', { n: props.name.importers })}</span>
      </Show>
    </button>
  )
}

const ContractBody: Component<{ contract: Contract }> = (props) => (
  <Show when={props.contract.names.length} fallback={<div class="empty">{t('contract.empty')}</div>}>
    <div class="ct-names">
      <For each={props.contract.names.slice(0, MAX_NAMES)}>{(n) => <NameRow name={n} />}</For>
      <More total={props.contract.names.length} shown={MAX_NAMES} />
    </div>
    <h4>{t('contract.byImporter', { n: props.contract.importers.length })}</h4>
    <div class="ct-files">
      <For each={props.contract.importers.slice(0, MAX_FILES)}>
        {(group) => (
          <div class="ct-file" data-testid="contract-importer" data-file={group.file}>
            <div class="ct-file-hd" title={group.file}>
              <PathLabel path={group.file} />
            </div>
            <For each={group.imports}>
              {(imp) => (
                <div class="ct-site" data-line={imp.line}>
                  <span class="ct-line">:{imp.line}</span>
                  <Show when={imp.isTypeOnly}>
                    <TypeBadge />
                  </Show>
                  <For each={imp.names}>
                    {(n) => {
                      const target = (): CodeTarget => ({
                        name: n.name,
                        signature: null,
                        decl: n.decl,
                        site: { kind: 'import', path: imp.file, line: imp.line },
                      })
                      return (
                        <button
                          class={`ct-chip${n.isTypeOnly ? ' ct-chip-type' : ''}${n.decl ? '' : ' ct-chip-lost'}${isOpen(target()) ? ' on' : ''}`}
                          data-testid="contract-site-name"
                          data-name={n.name}
                          data-decl={n.decl ? locText(n.decl) : undefined}
                          title={n.decl ? locText(n.decl) : t('contract.unresolved')}
                          onClick={() => openCode(target())}
                        >
                          {n.name}
                        </button>
                      )
                    }}
                  </For>
                </div>
              )}
            </For>
          </div>
        )}
      </For>
      <More total={props.contract.importers.length} shown={MAX_FILES} />
    </div>
  </Show>
)

export const ContractSection: Component<{ edges: readonly EdgeEnds[] }> = (props) => {
  const [contract] = createResource(() => props.edges, loadContract)
  return (
    <div class="det-section" data-testid="contract">
      <h4>{contract.state === 'ready' ? t('contract.title', { n: contract().names.length }) : t('contract.heading')}</h4>
      <Show when={props.edges.length > MAX_EDGES}>
        <p class="ct-note">{t('contract.partial', { shown: MAX_EDGES, total: props.edges.length })}</p>
      </Show>
      <Switch>
        <Match when={contract.state === 'errored'}>
          <div class="empty" data-testid="contract-unavailable">
            {t('contract.unavailable')}
          </div>
        </Match>
        <Match when={contract.state === 'ready' && contract()}>{(c) => <ContractBody contract={c()} />}</Match>
        <Match when={contract.loading}>
          <div class="empty">{t('contract.loading')}</div>
        </Match>
      </Switch>
    </div>
  )
}
