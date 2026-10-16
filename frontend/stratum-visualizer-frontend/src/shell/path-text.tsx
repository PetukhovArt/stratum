import { Component, For, Show } from 'solid-js'
import { splitPath } from '../render/labels'

/** Line breaks go after a `/`; inside a segment only when the segment alone is wider than the line. */
export const PathText: Component<{ path: string }> = (props) => {
  const segments = () => props.path.split('/')
  return (
    <For each={segments()}>
      {(segment, i) => (
        <Show when={i() < segments().length - 1} fallback={segment}>
          {segment}/<wbr />
        </Show>
      )}
    </For>
  )
}

/** One line: the directories give way to `…` before the file name does. */
export const PathLabel: Component<{ path: string }> = (props) => {
  const parts = () => splitPath(props.path)
  return (
    <span class="path-label">
      <Show when={parts().dir}>
        <span class="path-label-dir">{parts().dir}/</span>
      </Show>
      <span class="path-label-base">{parts().base}</span>
    </span>
  )
}
