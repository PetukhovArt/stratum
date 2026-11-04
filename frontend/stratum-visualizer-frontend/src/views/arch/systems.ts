import { createSignal } from 'solid-js'
import { Actor, ContractCaller, DeclaredLink, fetchSystems, SystemContract, SystemIndex, SystemInfo, ViaModule } from '../../lib/api'
import { Model } from '../../render/model'

export interface ChannelInUse {
  to: SystemInfo
  contracts: readonly SystemContract[]
  unattributedCalls: number
  via: readonly ViaModule[]
}

export const channelWeight = (c: { contracts: readonly unknown[]; via: readonly unknown[] }): number =>
  c.contracts.length || c.via.length

export interface PortLink {
  container: number
  system: string
  contracts: readonly SystemContract[]
  via: readonly ViaModule[]
}

export interface SystemsInModel {
  self: SystemInfo
  others: readonly SystemInfo[]
  internal: readonly SystemInfo[]
  external: readonly SystemInfo[]
  actors: readonly Actor[]
  links: readonly DeclaredLink[]
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

const byWeightDesc = (a: PortLink, b: PortLink): number => channelWeight(b) - channelWeight(a)

type Reach = Pick<ContractCaller, 'module' | 'features'>

const portLinksOf = (m: Model, channels: readonly ChannelInUse[]): PortLink[] => {
  const links: PortLink[] = []
  for (const channel of channels) {
    const unitOf = (r: Reach) => (m.modules.has(r.module) ? m.unitOf(r.module) : null)
    const containers = new Set([
      ...channel.contracts.flatMap((c) => c.callers.flatMap((caller) => unitOf(caller) ?? [])),
      ...channel.via.flatMap((v) => unitOf(v) ?? []),
    ])
    for (const container of [...containers].sort((a, b) => a - b)) {
      const contracts = narrowed(channel.contracts, (caller) => unitOf(caller) === container)
      const via = channel.via.filter((v) => unitOf(v) === container)
      links.push({ container, system: channel.to.id, contracts, via })
    }
  }
  return links.sort(byWeightDesc)
}

const shownBy = (m: Model): ((caller: Reach) => boolean) => {
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
    return [{ to, contracts, unattributedCalls: channel.unattributedCalls, via: channel.via.filter(isShown) }]
  })
  const portLinks = portLinksOf(m, channels)
  return {
    self: index.self,
    others: index.others,
    internal: index.others.filter((o) => !o.isExternal),
    external: index.others.filter((o) => o.isExternal),
    actors: index.actors,
    links: index.links,
    channels,
    channelTo: (system) => channels.find((c) => c.to.id === system) ?? null,
    portLinks,
    portLink: (container, system) => portLinks.find((l) => l.container === container && l.system === system) ?? null,
  }
}

const cache = new WeakMap<SystemIndex, WeakMap<Model, SystemsInModel>>()

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

export const systemIndex = (): SystemIndex | null => {
  requestSystems()
  return index()
}

export const systemsIn = (m: Model): SystemsInModel | null => {
  const found = systemIndex()
  return found ? systemsOf(found, m) : null
}
