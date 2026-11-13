import { Actor, DeclaredLink, SystemInfo } from '../../lib/api'
import { Bounds } from '../../render/viewport'
import { Curve, curveBetween, Point, Rect } from '../../render/route'
import { channelWeight, ChannelInUse, SystemsInModel } from './systems'

const SYSTEM = { selfW: 240, selfH: 96, w: 220, h: 76, gap: 170 } as const
const ACTOR = { w: 210, h: 74, gap: 40 } as const
const EXTERNAL_GAP = 50
const ZONE = { pad: 22, head: 30, gap: 84 } as const
const MARGIN = 30

type Size = Pick<Rect, 'w' | 'h'>

export type ZoneKind = 'actors' | 'system' | 'external'

export interface Zone extends Rect {
  kind: ZoneKind
}

export interface SystemBox extends Rect {
  system: SystemInfo
  isSelf: boolean
}

export interface ActorBox extends Rect {
  actor: Actor
}

export interface ChannelArrow {
  channel: ChannelInUse
  d: string
  label: Point
}

export interface LinkArrow {
  link: DeclaredLink
  d: string
  label: Point
}

export interface SystemLayout {
  zones: readonly Zone[]
  actors: readonly ActorBox[]
  boxes: readonly SystemBox[]
  arrows: readonly ChannelArrow[]
  links: readonly LinkArrow[]
  bounds: Bounds
}

export const zoneTitleBelow = (kind: ZoneKind): boolean => kind === 'external'

const SIDE ={ clearance: 3, lift: 8, minOverlap: 24 } as const

const connect = (a: Rect, b: Rect): Curve => {
  const top = Math.max(a.y, b.y)
  const bottom = Math.min(a.y + a.h, b.y + b.h)
  if (bottom - top < SIDE.minOverlap) return curveBetween(a, b)
  const y = (top + bottom) / 2
  const isRightward = b.x > a.x
  const from = isRightward ? a.x + a.w : a.x
  const to = isRightward ? b.x - SIDE.clearance : b.x + b.w + SIDE.clearance
  return { d: `M${from},${y} L${to},${y}`, label: { x: (from + to) / 2, y: y - SIDE.lift } }
}

const rowWidth =(n: number, w: number, gap: number): number => Math.max(0, n * w + (n - 1) * gap)

const sizeOf = (isSelf: boolean): Size => (isSelf ? { w: SYSTEM.selfW, h: SYSTEM.selfH } : { w: SYSTEM.w, h: SYSTEM.h })

const centreX = (r: Rect): number => r.x + r.w / 2

const zoneAround = (kind: ZoneKind, boxes: readonly Rect[]): Zone => {
  const x = Math.min(...boxes.map((b) => b.x)) - ZONE.pad
  const isTitleBelow = zoneTitleBelow(kind)
  const y = Math.min(...boxes.map((b) => b.y)) - (isTitleBelow ? ZONE.pad : ZONE.head)
  const right = Math.max(...boxes.map((b) => b.x + b.w)) + ZONE.pad
  const bottom = Math.max(...boxes.map((b) => b.y + b.h)) + (isTitleBelow ? ZONE.head : ZONE.pad)
  return { kind, x, y, w: right - x, h: bottom - y }
}

const selfInMiddle = (self: SystemInfo, internal: readonly SystemInfo[]): SystemInfo[] => {
  const left = internal.filter((_, i) => i % 2 === 0).reverse()
  const right = internal.filter((_, i) => i % 2 === 1)
  return [...left, self, ...right]
}

const rowTopBelow = (zone: Zone | null): number => (zone ? zone.y + zone.h + ZONE.gap + ZONE.head : ZONE.head)

const sourcesOf = (s: SystemsInModel, id: string): string[] => [
  ...s.channels.filter((c) => c.to.id === id && channelWeight(c)).map(() => s.self.id),
  ...s.links.filter((l) => l.to === id).map((l) => l.from),
]

export const layoutSystems = (s: SystemsInModel): SystemLayout => {
  const actorRowW = rowWidth(s.actors.length, ACTOR.w, ACTOR.gap)
  const actors = s.actors.map(
    (actor, i): ActorBox => ({ actor, x: i * (ACTOR.w + ACTOR.gap), y: ZONE.head, w: ACTOR.w, h: ACTOR.h }),
  )
  const actorZone = actors.length ? zoneAround('actors', actors) : null

  const rowTop = rowTopBelow(actorZone)
  let x = 0
  const boxes = selfInMiddle(s.self, s.internal).map((system): SystemBox => {
    const isSelf = system.id === s.self.id
    const size = sizeOf(isSelf)
    const box = { system, isSelf, x, y: rowTop + (SYSTEM.selfH - size.h) / 2, ...size }
    x += size.w + SYSTEM.gap
    return box
  })
  const self = boxes.find((b) => b.isSelf) ?? boxes[0]
  const shiftActors = centreX(self) - actorRowW / 2
  for (const a of actors) a.x += shiftActors
  if (actorZone) actorZone.x += shiftActors
  const systemZone = zoneAround('system', boxes)

  const centreOf = new Map<string, number>([
    ...boxes.map((b): [string, number] => [b.system.id, centreX(b)]),
    ...actors.map((a): [string, number] => [a.actor.id, centreX(a)]),
  ])
  const pull = (system: SystemInfo): number => {
    const xs = sourcesOf(s, system.id).flatMap((id) => centreOf.get(id) ?? [])
    return xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : centreX(systemZone)
  }
  const pulled = s.external.map((system) => ({ system, at: pull(system) })).sort((a, b) => a.at - b.at)
  const extRowW = rowWidth(pulled.length, SYSTEM.w, EXTERNAL_GAP)
  const extCentre = pulled.length ? pulled.reduce((sum, p) => sum + p.at, 0) / pulled.length : 0
  const extLeft = extCentre - extRowW / 2
  const extTop = rowTopBelow(systemZone)
  const externals = pulled.map(
    ({ system }, i): SystemBox => ({
      system,
      isSelf: false,
      x: extLeft + i * (SYSTEM.w + EXTERNAL_GAP),
      y: extTop,
      w: SYSTEM.w,
      h: SYSTEM.h,
    }),
  )

  const zones: Zone[] = [...(actorZone ? [actorZone] : []), systemZone]
  if (externals.length) zones.push(zoneAround('external', externals))

  const all = [...boxes, ...externals]
  const byId = new Map<string, Rect>([
    ...all.map((b): [string, Rect] => [b.system.id, b]),
    ...actors.map((a): [string, Rect] => [a.actor.id, a]),
  ])
  const arrows = s.channels.flatMap((channel): ChannelArrow[] => {
    const to = byId.get(channel.to.id)
    return to && channelWeight(channel) ? [{ channel, ...connect(self, to) }] : []
  })
  const links = s.links.flatMap((link): LinkArrow[] => {
    const from = byId.get(link.from)
    const to = byId.get(link.to)
    return from && to ? [{ link, ...connect(from, to) }] : []
  })

  const left = Math.min(...zones.map((z) => z.x)) - MARGIN
  const topEdge = Math.min(...zones.map((z) => z.y)) - MARGIN
  const right = Math.max(...zones.map((z) => z.x + z.w)) + MARGIN
  const bottom = Math.max(...zones.map((z) => z.y + z.h)) + MARGIN
  return {
    zones,
    actors,
    boxes: all,
    arrows,
    links,
    bounds: { x: left, y: topEdge, w: right - left, h: bottom - topEdge },
  }
}
