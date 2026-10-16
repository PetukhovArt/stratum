export interface Plural {
  one: string
  few?: string
  many?: string
  other: string
}

export type Message = string | Plural

export type MessagesOf<T> = { [K in keyof T]: Message }
