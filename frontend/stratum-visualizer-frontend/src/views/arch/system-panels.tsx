import { Component, For, Match, Show, Switch } from 'solid-js'
import { codeTarget, CodeTarget, openCode } from '../../contract/code'
import { t } from '../../i18n'
import { ContractCaller, Loc, SystemContract, SystemInfo, ViaModule } from '../../lib/api'
import { Model } from '../../render/model'
import { go, nav, set } from '../../shell/nav'
import { PathLabel } from '../../shell/path-text'
import { contractCode, goToLevel, moduleCode } from './actions'
import { Boxes } from './boxes'
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
    <p class="arch-note arch-unattributed" data-testid="arch-unattributed" title={t('arch.unattributedTitle')}>
      {t('arch.unattributed', { n: props.n })}
    </p>
  </Show>
)

const ViaList: Component<{ model: Model; via: readonly ViaModule[] }> = (props) => (
  <div class="arch-via">
    <For each={props.via}>
      {(v) => {
        const target = () => moduleCode(props.model, v.module)
        const path = () => props.model.modules.get(v.module)?.path ?? String(v.module)
        return (
          <button
            class={`arch-caller${isOpen(target()) ? ' on' : ''}`}
            data-testid="arch-via-module"
            data-module={v.module}
            title={path()}
            onClick={() => openCode(target())}
            onDblClick={() => go({ view: 'focus', focus: v.module })}
          >
            <PathLabel path={path()} />
          </button>
        )
      }}
    </For>
  </div>
)

const ViaSection: Component<{ model: Model; via: readonly ViaModule[] }> = (props) => (
  <Show when={props.via.length}>
    <div class="det-section" data-testid="arch-via">
      <h4>{t('arch.channel.viaTitle', { n: props.via.length })}</h4>
      <p class="arch-note">{t('arch.channel.viaHint')}</p>
      <ViaList model={props.model} via={props.via} />
    </div>
  </Show>
)

const ChannelCard: Component<{ model: Model; systems: SystemsInModel; channel: ChannelInUse }> = (props) => (
  <div data-testid="arch-channel-details" data-from={props.systems.self.id} data-to={props.channel.to.id}>
    <div class="det-section">
      <h4>
        {props.systems.self.name} → {props.channel.to.name} ·{' '}
        {props.channel.contracts.length || !props.channel.via.length
          ? t('arch.channel.contracts', { n: props.channel.contracts.length })
          : t('arch.channel.via', { n: props.channel.via.length })}
      </h4>
      <Show when={props.channel.declared}>
        {(label) => (
          <p class="arch-note" title={t('arch.link.declared')}>
            {label()}
          </p>
        )}
      </Show>
      <Unattributed n={props.channel.unattributedCalls} />
      <Show
        when={props.channel.contracts.length}
        fallback={
          <Show when={!props.channel.via.length}>
            <div class="empty">{t('arch.channel.none')}</div>
          </Show>
        }
      >
        <ContractList model={props.model} contracts={props.channel.contracts} />
      </Show>
    </div>
    <ViaSection model={props.model} via={props.channel.via} />
  </div>
)

const ChannelRows: Component<{ systems: SystemsInModel }> = (props) => (
  <div class="arch-rows">
    <For each={props.systems.channels}>
      {(c) => (
        <button
          class="arch-row"
          data-testid="arch-channel-row"
          data-from={props.systems.self.id}
          data-to={c.to.id}
          onClick={() => set({ sel: { kind: 'channel', to: c.to.id } })}
        >
          <span class="nm mono">
            {props.systems.self.name} → {c.to.name}
          </span>
          <span class="n">{c.contracts.length || (c.via.length ? t('arch.channel.via', { n: c.via.length }) : 0)}</span>
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
        <Show when={props.system.description}>{(d) => <p class="arch-note">{d()}</p>}</Show>
        <Switch>
          <Match when={isSelf()}>
            <p class="arch-note">{t('arch.system.self', { n: props.model.modules.size })}</p>
            <div class="arch-btn-row">
              <button class="arch-btn" data-testid="arch-enter-system" onClick={() => goToLevel(props.model, ARCH_CONTAINERS)}>
                {t('arch.enterSystem')} <kbd>Enter</kbd>
              </button>
            </div>
          </Match>
          <Match when={props.system.unavailableReason}>
            {(reason) => (
              <p class="arch-note arch-muted" data-testid="arch-system-unavailable">
                {t('arch.system.unavailableReason', { reason: reason() })}
              </p>
            )}
          </Match>
          <Match when={props.system.isExternal && !props.system.contractsTotal}>
            <p class="arch-note" data-testid="arch-system-external-note">
              {t('arch.system.external')}
            </p>
          </Match>
          <Match when={!isSelf()}>
            <p class="arch-note">{t('arch.system.contracts', { n: props.system.contractsTotal })}</p>
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
    <p class="arch-note">{t('arch.systems.hint')}</p>
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

export const PortDetails: Component<{ model: Model; systems: SystemsInModel; boxes: Boxes }> = (props) => {
  const flow = () => {
    const sel = nav().sel
    if (sel?.kind !== 'port') return null
    const links = props.systems.portLinks.filter(
      (l) => l.system === sel.system && props.boxes.boxOfUnit.get(l.container) === sel.box,
    )
    return links.length ? { box: sel.box, system: sel.system, links } : null
  }
  const system = () => {
    const sel = nav().sel
    return sel?.kind === 'system' ? systemById(props.systems, sel.id) : null
  }
  return (
    <Switch>
      <Match when={flow()}>
        {(f) => (
          <div data-testid="arch-port-details" data-box={f().box} data-system={f().system}>
            <For each={f().links}>
              {(l) => (
                <div class="det-section" data-testid="arch-port-unit" data-container={l.container}>
                  <h4>
                    {props.model.unitLabel(l.container)} → {systemById(props.systems, l.system)?.name ?? l.system} ·{' '}
                    {t('arch.channel.contracts', { n: l.contracts.length })}
                  </h4>
                  <ContractList model={props.model} contracts={l.contracts} />
                  <Show when={l.via.length}>
                    <h4>{t('arch.channel.viaTitle', { n: l.via.length })}</h4>
                    <ViaList model={props.model} via={l.via} />
                  </Show>
                </div>
              )}
            </For>
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
              class={`arch-ol-unit${selected() === s.id ? ' on' : ''}${s.unavailableReason ? ' arch-muted' : ''}`}
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
