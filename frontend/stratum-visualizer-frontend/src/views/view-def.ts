import { Component } from 'solid-js'
import { MessageKey } from '../i18n'
import { Model } from '../render/model'
import { NavState, ViewId } from '../shell/nav'

export interface ViewProps {
  model: Model
  isViolationsUnavailable: boolean
}

type VariantKey = 'focusVariant' | 'dsmLevel' | 'mrVariant' | 'mapVariant' | 'setVariant'

export type Variants = {
  [K in VariantKey]: { key: K; options: readonly { id: NavState[K]; title: MessageKey }[] }
}[VariantKey]

export interface ViewDef {
  id: ViewId
  key: string
  title: MessageKey
  Stage: Component<ViewProps>
  Outline: Component<ViewProps>
  Details: Component<ViewProps>
  variants?: Variants
  isAvailable?: () => boolean
}
