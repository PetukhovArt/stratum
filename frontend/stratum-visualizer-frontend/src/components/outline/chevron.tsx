import { Component } from 'solid-js'

export const Chevron: Component<{ isCollapsed: boolean; onClick?: (e: MouseEvent) => void }> = (
  props,
) => (
  <span
    class={`ot-chev ${props.isCollapsed ? 'is-collapsed' : 'is-open'}`}
    aria-hidden="true"
    onClick={props.onClick}
  >
    <svg width="8" height="8" viewBox="0 0 8 8">
      <path d="M2 1 L6 4 L2 7 Z" fill="currentColor" />
    </svg>
  </span>
)
