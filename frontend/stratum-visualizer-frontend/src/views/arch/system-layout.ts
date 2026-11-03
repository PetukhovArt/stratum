import { Actor, DeclaredLink, SystemInfo } from '../../lib/api'
import { Bounds } from '../../render/viewport'
import { BlocksLayout, curveBetween, Point, Rect, Size } from '../map/layout'
import { channelWeight, ChannelInUse, PortLink, SystemsInModel } from './systems'

const SYSTEM = { selfW: 240, selfH: 96, w: 220, h: 76 } as const
const ACTOR = { w: 200, h: 66, gap: 28 } as const
const ZONE = { pad: 22, head: 30, gap: 70, colGap: 90, rowGap: 60, extGap: 24 } as const
const PORT = { w: 160, h: 46, gap: 18, margin: 24 } as const
const MARGIN = 30
const COLUMNS = 2

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

const stackHeight = (n: number, h: number, gap: number): number => Math.max(0, n * h + (n - 1) * gap)

const sizeOf = (isSelf: boolean): Size => (isSelf ? { w: SYSTEM.selfW, h: SYSTEM.selfH } : { w: SYSTEM.w, h: SYSTEM.h })

const zoneAround = (kind: ZoneKind, boxes: readonly Rect[]): Zone => {
  const x = Math.min(...boxes.map((b) => b.x)) - ZONE.pad
  const y = Math.min(...boxes.map((b) => b.y)) - ZONE.head
  const right = Math.max(...boxes.map((b) => b.x + b.w)) + ZONE.pad
  const bottom = Math.max(...boxes.map((b) => b.y + b.h)) + ZONE.pad
  return { kind, x, y, w: right - x, h: bottom - y }
}

export const layoutSystems = (s: SystemsInModel): SystemLayout => {
  const hasActors = s.actors.length > 0
  const top = hasActors ? ACTOR.h + ZONE.head + ZONE.pad * 2 + ZONE.gap : 0

  const members = [s.self, ...s.internal]
  const cellW = SYSTEM.selfW + ZONE.colGap
  const cellH = SYSTEM.selfH + ZONE.rowGap
  const columns = Math.min(COLUMNS, members.length)
  const boxes: SystemBox[] = members.map((system, i) => {
    const isSelf = i === 0
    const size = sizeOf(isSelf)
    return {
      system,
      isSelf,
      x: (i % columns) * cellW + (SYSTEM.selfW - size.w) / 2,
      y: top + ZONE.head + Math.floor(i / columns) * cellH + (SYSTEM.selfH - size.h) / 2,
      ...size,
    }
  })
  const systemZone = zoneAround('system', boxes)

  const extX = systemZone.x + systemZone.w + ZONE.gap + ZONE.pad
  const extTop = top + ZONE.head
  const externals = s.external.map(
    (system, i): SystemBox => ({
      system,
      isSelf: false,
      x: extX,
      y: extTop + i * (SYSTEM.h + ZONE.extGap),
      w: SYSTEM.w,
      h: SYSTEM.h,
    }),
  )
  const zones: Zone[] = [systemZone]
  if (externals.length) zones.push(zoneAround('external', externals))

  const rowW = stackHeight(s.actors.length, ACTOR.w, ACTOR.gap)
  const actorsX = systemZone.x + ZONE.pad + Math.max(0, (systemZone.w - ZONE.pad * 2 - rowW) / 2)
  const actors = s.actors.map(
    (actor, i): ActorBox => ({ actor, x: actorsX + i * (ACTOR.w + ACTOR.gap), y: ZONE.head, w: ACTOR.w, h: ACTOR.h }),
  )
  if (actors.length) zones.unshift(zoneAround('actors', actors))

  const all = [...boxes, ...externals]
  const byId = new Map<string, Rect>([
    ...all.map((b): [string, Rect] => [b.system.id, b]),
    ...actors.map((a): [string, Rect] => [a.actor.id, a]),
  ])
  const self = boxes[0]
  const arrows = s.channels.flatMap((channel): ChannelArrow[] => {
    const to = byId.get(channel.to.id)
    return to && channelWeight(channel) ? [{ channel, ...curveBetween(self, to) }] : []
  })
  const links = s.links.flatMap((link): LinkArrow[] => {
    const from = byId.get(link.from)
    const to = byId.get(link.to)
    return from && to ? [{ link, ...curveBetween(from, to) }] : []
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

export interface PortBox extends Rect {
  system: SystemInfo
}

export interface PortArrow {
  link: PortLink
  d: string
  label: Point
}

export interface PortsLayout {
  ports: readonly PortBox[]
  arrows: readonly PortArrow[]
}

export const portsSlot = (s: SystemsInModel): Size => ({
  w: PORT.w + PORT.margin * 2,
  h: stackHeight(s.others.length, PORT.h, PORT.gap) + PORT.margin * 2,
})

export const layoutPorts = (layout: BlocksLayout, slot: Rect, s: SystemsInModel): PortsLayout => {
  const ports = s.others.map(
    (system, i): PortBox => ({
      system,
      x: slot.x + PORT.margin,
      y: slot.y + PORT.margin + i * (PORT.h + PORT.gap),
      w: PORT.w,
      h: PORT.h,
    }),
  )
  const byId = new Map(ports.map((p) => [p.system.id, p]))
  const arrows = s.portLinks.flatMap((link): PortArrow[] => {
    const from = layout.boxes.get(link.container)
    const to = byId.get(link.system)
    return from && to ? [{ link, ...curveBetween(from, to) }] : []
  })
  return { ports, arrows }
}
