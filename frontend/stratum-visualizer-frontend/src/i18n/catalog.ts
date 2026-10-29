import { en as contractEn, ru as contractRu } from '../contract/messages'
import { en as archEn, ru as archRu } from '../views/arch/messages'
import { en as featuresEn, ru as featuresRu } from '../features/messages'
import { en as shellEn, ru as shellRu } from '../shell/messages'
import { en as benchEn, ru as benchRu } from '../views/bench/messages'
import { en as dsmEn, ru as dsmRu } from '../views/dsm/messages'
import { en as viewsEn, ru as viewsRu } from '../views/messages'
import { en as focusEn, ru as focusRu } from '../views/focus/messages'
import { en as mapEn, ru as mapRu } from '../views/map/messages'
import { en as mrEn, ru as mrRu } from '../views/mr/messages'
import { en as setEn, ru as setRu } from '../views/set/messages'
import type { MessagesOf } from './types'

export const en = {
  ...shellEn,
  ...viewsEn,
  ...dsmEn,
  ...focusEn,
  ...mrEn,
  ...mapEn,
  ...setEn,
  ...archEn,
  ...benchEn,
  ...contractEn,
  ...featuresEn,
} as const

export type MessageKey = keyof typeof en
export type Messages = MessagesOf<typeof en>

export const ru: Messages = {
  ...shellRu,
  ...viewsRu,
  ...dsmRu,
  ...focusRu,
  ...mrRu,
  ...mapRu,
  ...setRu,
  ...archRu,
  ...benchRu,
  ...contractRu,
  ...featuresRu,
}
