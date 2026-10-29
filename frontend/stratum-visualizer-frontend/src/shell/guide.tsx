import { Component, createSignal, For } from 'solid-js'
import { lang } from '../i18n'
import { GUIDE } from './guide-content'

const [isGuideOpen, setGuideOpen] = createSignal(false)

export { isGuideOpen }
export const openGuide = (): void => {
  setGuideOpen(true)
}
export const closeGuide = (): boolean => {
  if (!isGuideOpen()) return false
  setGuideOpen(false)
  return true
}

export const Guide: Component = () => {
  const doc = () => GUIDE[lang()]
  return (
    <div class="pal-scrim guide-scrim" onClick={(e) => e.target === e.currentTarget && closeGuide()}>
      <article class="guide" role="dialog" aria-modal="true" aria-label={doc().title} data-testid="guide">
        <header class="guide-hd">
          <h1>{doc().title}</h1>
          <button class="guide-x" data-testid="guide-close" aria-label={doc().close} title={`${doc().close} (Esc)`} onClick={closeGuide}>
            ×
          </button>
        </header>
        <div class="guide-body">
          <For each={doc().intro}>{(p) => <p>{p}</p>}</For>

          <h2>{doc().run.title}</h2>
          <ul class="guide-run">
            <For each={doc().run.lines}>{(line) => <li>{line}</li>}</For>
          </ul>

          <h2>{doc().views.title}</h2>
          <dl class="guide-views">
            <For each={doc().views.items}>
              {(v) => (
                <>
                  <dt>
                    <kbd>{v.key}</kbd> {v.name}
                  </dt>
                  <dd>{v.text}</dd>
                </>
              )}
            </For>
          </dl>

          <h2>{doc().cases.title}</h2>
          <For each={doc().cases.items}>
            {(c) => (
              <section class="guide-case">
                <h3>{c.goal}</h3>
                <ol>
                  <For each={c.steps}>{(s) => <li>{s}</li>}</For>
                </ol>
              </section>
            )}
          </For>

          <h2>{doc().controls.title}</h2>
          <table class="guide-keys">
            <tbody>
              <For each={doc().controls.items}>
                {([input, action]) => (
                  <tr>
                    <td>
                      <kbd>{input}</kbd>
                    </td>
                    <td>{action}</td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </article>
    </div>
  )
}
