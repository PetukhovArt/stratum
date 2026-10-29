import { SystemInfo } from '../../lib/api'
import { Bounds } from '../../render/viewport'
import { BlocksLayout, curveBetween, Point, Rect, Size } from '../map/layout'
import { ChannelInUse, PortLink, SystemsInModel } from './systems'

const SYSTEM = { selfW: 240, selfH: 110, w: 230, h: 76, gap: 24, column: 280, margin: 30 } as const
const PORT = { w: 160, h: 46, gap: 18, margin: 24 } as const

export interface SystemBox extends Rect {
  system: SystemInfo
  isSelf: boolean
}

export interface ChannelArrow {
  channel: ChannelInUse
  d: string
  label: Point
}

export interface SystemLayout {
  boxes: readonly SystemBox[]
  arrows: readonly ChannelArrow[]
  bounds: Bounds
}

const stackHeight = (n: number, h: number, gap: number): number => Math.max(0, n * h + (n - 1) * gap)

export const layoutSystems = (s: SystemsInModel): SystemLayout => {
  const column = stackHeight(s.others.length, SYSTEM.h, SYSTEM.gap)
  const height = Math.max(SYSTEM.selfH, column)
  const self: SystemBox = {
    system: s.self,
    isSelf: true,
    x: 0,
    y: (height - SYSTEM.selfH) / 2,
    w: SYSTEM.selfW,
    h: SYSTEM.selfH,
  }
  const top = (height - column) / 2
  const others = s.others.map(
    (system, i): SystemBox => ({
      system,
      isSelf: false,
      x: SYSTEM.selfW + SYSTEM.column,
      y: top + i * (SYSTEM.h + SYSTEM.gap),
      w: SYSTEM.w,
      h: SYSTEM.h,
    }),
  )
  const byId = new Map(others.map((b) => [b.system.id, b]))
  const arrows = s.channels.flatMap((channel): ChannelArrow[] => {
    const to = byId.get(channel.to.id)
    return to && channel.contracts.length ? [{ channel, ...curveBetween(self, to) }] : []
  })
  const m = SYSTEM.margin
  return {
    boxes: [self, ...others],
    arrows,
    bounds: { x: -m, y: -m, w: SYSTEM.selfW + SYSTEM.column + SYSTEM.w + m * 2, h: height + m * 2 },
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
