import { en as shellEn, ru as shellRu } from '../shell/messages'
import { en as dsmEn, ru as dsmRu } from '../views/dsm/messages'
import { en as viewsEn, ru as viewsRu } from '../views/messages'
import { en as focusEn, ru as focusRu } from '../views/focus/messages'
import { en as mrEn, ru as mrRu } from '../views/mr/messages'
import type { MessagesOf } from './types'

export const en = { ...shellEn, ...viewsEn, ...dsmEn, ...focusEn, ...mrEn } as const

export type MessageKey = keyof typeof en
export type Messages = MessagesOf<typeof en>

export const ru: Messages = { ...shellRu, ...viewsRu, ...dsmRu, ...focusRu, ...mrRu }
