import { Component, Show } from 'solid-js'

export const BudgetNote: Component<{ text: string; action?: { label: string; run: () => void } }> = (props) => (
  <div class="budget-note" data-testid="budget-note">
    {props.text}
    <Show when={props.action}>
      {(action) => (
        <button data-testid="budget-note-action" onClick={() => action().run()}>
          {action().label}
        </button>
      )}
    </Show>
  </div>
)
