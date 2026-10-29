import { createSignal } from 'solid-js'
import { ContractCaller, fetchSystems, SystemContract, SystemIndex, SystemInfo } from '../../lib/api'
import { Model } from '../../render/model'

export interface ChannelInUse {
  to: SystemInfo
  contracts: readonly SystemContract[]
  unattributedCalls: number
}

export interface PortLink {
  container: number
  system: string
  contracts: readonly SystemContract[]
}

export interface SystemsInModel {
  self: SystemInfo
  others: readonly SystemInfo[]
  channels: readonly ChannelInUse[]
  channelTo: (system: string) => ChannelInUse | null
  portLinks: readonly PortLink[]
  portLink: (container: number, system: string) => PortLink | null
}

const narrowed = (contracts: readonly SystemContract[], keeps: (caller: ContractCaller) => boolean): SystemContract[] =>
  contracts.flatMap((c) => {
    const callers = c.callers.filter(keeps)
    return callers.length ? [{ ...c, callers }] : []
  })

const byContractsDesc = (a: PortLink, b: PortLink): number => b.contracts.length - a.contracts.length

const portLinksOf = (m: Model, channels: readonly ChannelInUse[]): PortLink[] => {
  const links: PortLink[] = []
  for (const channel of channels) {
    const unitOf = (caller: ContractCaller) => (m.modules.has(caller.module) ? m.unitOf(caller.module) : null)
    const containers = new Set(channel.contracts.flatMap((c) => c.callers.flatMap((caller) => unitOf(caller) ?? [])))
    for (const container of [...containers].sort((a, b) => a - b)) {
      const contracts = narrowed(channel.contracts, (caller) => unitOf(caller) === container)
      links.push({ container, system: channel.to.id, contracts })
    }
  }
  return links.sort(byContractsDesc)
}

const shownBy = (m: Model): ((caller: ContractCaller) => boolean) => {
  const chosen = new Set(m.chosenFeatures)
  return (caller) =>
    chosen.size && caller.features
      ? caller.features.some((id) => chosen.has(id))
      : m.modules.has(caller.module)
}

const build = (index: SystemIndex, m: Model): SystemsInModel => {
  const byId = new Map(index.others.map((s) => [s.id, s]))
  const isShown = shownBy(m)
  const channels = index.channels.flatMap((channel): ChannelInUse[] => {
    const to = byId.get(channel.to)
    if (channel.from !== index.self.id || !to) return []
    const contracts = narrowed(channel.contracts, isShown)
    return [{ to, contracts, unattributedCalls: channel.unattributedCalls }]
  })
  const portLinks = portLinksOf(m, channels)
  return {
    self: index.self,
    others: index.others,
    channels,
    channelTo: (system) => channels.find((c) => c.to.id === system) ?? null,
    portLinks,
    portLink: (container, system) => portLinks.find((l) => l.container === container && l.system === system) ?? null,
  }
}

const cache = new WeakMap<SystemIndex, WeakMap<Model, SystemsInModel>>()

/**
 * A contract stays only with its shown callers, and only if one is left: with features chosen, a
 * caller one of them runs; otherwise a caller whose module the Model holds.
 */
export const systemsOf = (index: SystemIndex, m: Model): SystemsInModel => {
  const byModel = cache.get(index) ?? new WeakMap<Model, SystemsInModel>()
  cache.set(index, byModel)
  const hit = byModel.get(m)
  if (hit) return hit
  const made = build(index, m)
  byModel.set(m, made)
  return made
}

const [index, setIndex] = createSignal<SystemIndex | null>(null)
let isRequested = false

export const requestSystems = (): void => {
  if (isRequested) return
  isRequested = true
  fetchSystems().then(setIndex, () => setIndex(null))
}

/** Requests `/api/systems` on the first read; `null` until it answers, and for good when it fails or answers 404. */
export const systemIndex = (): SystemIndex | null => {
  requestSystems()
  return index()
}

export const systemsIn = (m: Model): SystemsInModel | null => {
  const found = systemIndex()
  return found ? systemsOf(found, m) : null
}
