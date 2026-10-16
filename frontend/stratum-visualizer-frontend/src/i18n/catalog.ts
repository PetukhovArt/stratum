import { en as shellEn, ru as shellRu } from '../shell/messages'
import { en as viewsEn, ru as viewsRu } from '../views/messages'
import type { MessagesOf } from './types'

export const en = { ...shellEn, ...viewsEn } as const

export type MessageKey = keyof typeof en
export type Messages = MessagesOf<typeof en>

export const ru: Messages = { ...shellRu, ...viewsRu }
