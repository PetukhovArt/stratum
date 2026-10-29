import { Component, For, Match, Show, Switch } from 'solid-js'
import { codeTarget, CodeTarget, openCode } from '../../contract/code'
import { t } from '../../i18n'
import { ContractCaller, Loc, SystemContract, SystemInfo } from '../../lib/api'
import { Model } from '../../render/model'
import { nav, set } from '../../shell/nav'
import { PathLabel } from '../../shell/path-text'
import { contractCode, goToLevel } from './actions'
import { ARCH_CONTAINERS } from './path'
import { ChannelInUse, SystemsInModel } from './systems'

const MAX_CONTRACTS = 80

const locText = (at: Loc): string => `${at.path}:${at.line}`

const isOpen = (target: CodeTarget): boolean => {
  const open = codeTarget()
  return open?.name === target.name && open.site?.path === target.site?.path && open.site?.line === target.site?.line
}

const callerText = (m: Model, caller: ContractCaller): string => {
  const at = `${m.modules.get(caller.module)?.path ?? caller.module}:${caller.line}`
  return caller.owner ? `${at} (${caller.owner})` : at
}

const ContractRow: Component<{ model: Model; contract: SystemContract }> = (props) => {
  const c = props.contract
  const target = () => contractCode(props.model, c)
  return (
    <div class="arch-ct">
      <button
        class={`ct-name${isOpen(target()) ? ' on' : ''}`}
        data-testid="arch-contract"
        data-id={c.id}
        data-decl={c.decl ? locText(c.decl) : undefined}
        title={c.decl ? locText(c.decl) : t('contract.unresolved')}
        onClick={() => openCode(target())}
      >
        <span class="ct-nm">{c.id}</span>
        <span class="ct-decl">
          <Show when={c.decl} fallback={<span class="ct-unresolved">{t('contract.unresolved')}</span>}>
            {(decl) => <PathLabel path={locText(decl())} />}
          </Show>
        </span>
      </button>
      <Show when={c.signature}>{(signature) => <div class="arch-sig mono">{signature()}</div>}</Show>
      <div class="arch-callers">
        <For each={c.callers}>
          {(caller) => {
            const callerTarget = () => contractCode(props.model, c, caller)
            return (
              <button
                class={`arch-caller${isOpen(callerTarget()) ? ' on' : ''}`}
                data-testid="arch-caller"
                data-module={caller.module}
                data-line={caller.line}
                title={callerText(props.model, caller)}
                onClick={() => openCode(callerTarget())}
              >
                <PathLabel path={callerText(props.model, caller)} />
              </button>
            )
          }}
        </For>
      </div>
    </div>
  )
}

const ContractList: Component<{ model: Model; contracts: readonly SystemContract[] }> = (props) => (
  <div class="arch-cts">
    <For each={props.contracts.slice(0, MAX_CONTRACTS)}>{(c) => <ContractRow model={props.model} contract={c} />}</For>
    <Show when={props.contracts.length > MAX_CONTRACTS}>
      <div class="empty">{t('contract.more', { n: props.contracts.length - MAX_CONTRACTS })}</div>
    </Show>
  </div>
)

const Unattributed: Component<{ n: number }> = (props) => (
  <Show when={props.n}>
    <p class="map-note arch-unattributed" data-testid="arch-unattributed" title={t('arch.unattributedTitle')}>
      {t('arch.unattributed', { n: props.n })}
    </p>
  </Show>
)

const ChannelCard: Component<{ model: Model; systems: SystemsInModel; channel: ChannelInUse }> = (props) => (
  <div data-testid="arch-channel-details" data-from={props.systems.self.id} data-to={props.channel.to.id}>
    <div class="det-section">
      <h4>
        {props.systems.self.name} → {props.channel.to.name} · {t('arch.channel.contracts', { n: props.channel.contracts.length })}
      </h4>
      <Unattributed n={props.channel.unattributedCalls} />
      <Show when={props.channel.contracts.length} fallback={<div class="empty">{t('arch.channel.none')}</div>}>
        <ContractList model={props.model} contracts={props.channel.contracts} />
      </Show>
    </div>
  </div>
)

const ChannelRows: Component<{ systems: SystemsInModel }> = (props) => (
  <div class="map-rows">
    <For each={props.systems.channels}>
      {(c) => (
        <button
          class="map-row"
          data-testid="arch-channel-row"
          data-from={props.systems.self.id}
          data-to={c.to.id}
          onClick={() => set({ sel: { kind: 'channel', to: c.to.id } })}
        >
          <span class="nm mono">
            {props.systems.self.name} → {c.to.name}
          </span>
          <span class="n">{c.contracts.length}</span>
        </button>
      )}
    </For>
  </div>
)

const SystemCard: Component<{ model: Model; systems: SystemsInModel; system: SystemInfo }> = (props) => {
  const isSelf = () => props.system.id === props.systems.self.id
  const channel = () => props.systems.channelTo(props.system.id)
  return (
    <div data-testid="arch-system-details" data-id={props.system.id}>
      <div class="det-section">
        <h4>{props.system.name}</h4>
        <Switch>
          <Match when={isSelf()}>
            <p class="map-note">{t('arch.system.self', { n: props.model.modules.size })}</p>
            <div class="map-btn-row">
              <button class="map-btn" data-testid="arch-enter-system" onClick={() => goToLevel(props.model, ARCH_CONTAINERS)}>
                {t('arch.enterSystem')} <kbd>Enter</kbd>
              </button>
            </div>
          </Match>
          <Match when={props.system.unavailableReason}>
            {(reason) => (
              <p class="map-note arch-muted" data-testid="arch-system-unavailable">
                {t('arch.system.unavailableReason', { reason: reason() })}
              </p>
            )}
          </Match>
          <Match when={!isSelf()}>
            <p class="map-note">{t('arch.system.contracts', { n: props.system.contractsTotal })}</p>
          </Match>
        </Switch>
      </div>
      <Show when={isSelf()}>
        <div class="det-section">
          <h4>{t('arch.systems.channels')}</h4>
          <ChannelRows systems={props.systems} />
        </div>
      </Show>
      <Show when={channel()}>{(c) => <ChannelCard model={props.model} systems={props.systems} channel={c()} />}</Show>
    </div>
  )
}

const SystemsOverview: Component<{ systems: SystemsInModel }> = (props) => (
  <>
    <p class="map-note">{t('arch.systems.hint')}</p>
    <div class="det-section">
      <h4>{t('arch.systems.channels')}</h4>
      <Show when={props.systems.channels.length} fallback={<div class="empty">{t('arch.systems.noChannels')}</div>}>
        <ChannelRows systems={props.systems} />
      </Show>
    </div>
  </>
)

const systemById = (s: SystemsInModel, id: string): SystemInfo | null =>
  id === s.self.id ? s.self : (s.others.find((o) => o.id === id) ?? null)

export const SystemDetails: Component<{ model: Model; systems: SystemsInModel }> = (props) => {
  const system = () => {
    const sel = nav().sel
    return sel?.kind === 'system' ? systemById(props.systems, sel.id) : null
  }
  const channel = () => {
    const sel = nav().sel
    return sel?.kind === 'channel' ? props.systems.channelTo(sel.to) : null
  }
  return (
    <Switch fallback={<SystemsOverview systems={props.systems} />}>
      <Match when={channel()}>{(c) => <ChannelCard model={props.model} systems={props.systems} channel={c()} />}</Match>
      <Match when={system()}>{(s) => <SystemCard model={props.model} systems={props.systems} system={s()} />}</Match>
    </Switch>
  )
}

export const PortDetails: Component<{ model: Model; systems: SystemsInModel }> = (props) => {
  const link = () => {
    const sel = nav().sel
    return sel?.kind === 'port' ? props.systems.portLink(sel.container, sel.system) : null
  }
  const system = () => {
    const sel = nav().sel
    return sel?.kind === 'system' ? systemById(props.systems, sel.id) : null
  }
  return (
    <Switch>
      <Match when={link()}>
        {(l) => (
          <div data-testid="arch-port-details" data-container={l().container} data-system={l().system}>
            <div class="det-section">
              <h4>
                {props.model.unitLabel(l().container)} → {systemById(props.systems, l().system)?.name ?? l().system} ·{' '}
                {t('arch.channel.contracts', { n: l().contracts.length })}
              </h4>
              <ContractList model={props.model} contracts={l().contracts} />
            </div>
          </div>
        )}
      </Match>
      <Match when={system()}>{(s) => <SystemCard model={props.model} systems={props.systems} system={s()} />}</Match>
    </Switch>
  )
}

export const isPortSelection = (): boolean => {
  const kind = nav().sel?.kind
  return kind === 'port' || kind === 'system'
}

export const SystemOutline: Component<{ systems: SystemsInModel }> = (props) => {
  const selected = () => {
    const sel = nav().sel
    return sel?.kind === 'system' ? sel.id : null
  }
  return (
    <aside class="panel">
      <header class="panel-hd">
        <span class="panel-title">{t('arch.systems.title')}</span>
        <span class="panel-count">{props.systems.others.length + 1}</span>
      </header>
      <div class="outline-tree">
        <For each={[props.systems.self, ...props.systems.others]}>
          {(s) => (
            <div
              class={`map-ol-unit${selected() === s.id ? ' on' : ''}${s.unavailableReason ? ' arch-muted' : ''}`}
              data-testid="arch-outline-system"
              data-id={s.id}
              title={s.unavailableReason ?? s.name}
              onClick={() => set({ sel: { kind: 'system', id: s.id } })}
            >
              <span class="nm">{s.name}</span>
            </div>
          )}
        </For>
      </div>
    </aside>
  )
}
