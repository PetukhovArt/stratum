import { Component, createMemo, For, Show } from 'solid-js'
import { FeatureFilter } from '../features/feature-filter'
import { lang, LANGS, setLang, t } from '../i18n'
import type { Feature } from '../lib/api'
import { Model } from '../render/model'
import { Severity } from '../types'
import { availableViews } from '../views'
import { go, nav } from './nav'

export const Header: Component<{
  model: Model
  features: readonly Feature[]
  chosenFeatures: readonly string[]
  onOpenPalette: () => void
}> = (props) => {
  const counts = createMemo(() => {
    let err = 0
    let warn = 0
    for (const v of props.model.violations) {
      if (v.severity === Severity.Error) err++
      else if (v.severity === Severity.Warning) warn++
    }
    return { err, warn }
  })

  return (
    <header class="hd">
      <div class="hd-l">
        <div class="brand">
          <span class="brand-mark" />
          {t('shell.brand')} <span class="brand-dim">{t('shell.brandDim')}</span>
        </div>
        <span class="hd-div" />
        <span class="hd-proj" data-testid="project">
          {props.model.project}
        </span>
        <span class="hd-stats">
          <span>{t('shell.stats.modules', { n: props.model.modules.size })}</span>
          <span>{t('shell.stats.edges', { n: props.model.edges.length })}</span>
          <span class="c-err">{t('shell.stats.err', { n: counts().err })}</span>
          <span class="c-warn">{t('shell.stats.warn', { n: counts().warn })}</span>
        </span>
      </div>
      <nav class="tabs">
        <For each={availableViews()}>
          {(v) => (
            <button
              class={`tab ${nav().view === v.id ? 'on' : ''}`}
              data-testid={`tab-${v.id}`}
              onClick={() => go({ view: v.id })}
            >
              {t(v.title)}
              <kbd>{v.key}</kbd>
            </button>
          )}
        </For>
      </nav>
      <div class="hd-r">
        <Show when={props.features.length}>
          <FeatureFilter features={props.features} chosen={props.chosenFeatures} />
        </Show>
        <div class="seg lang">
          <For each={LANGS}>
            {(l) => (
              <button class={lang() === l ? 'on' : ''} onClick={() => setLang(l)}>
                {l.toUpperCase()}
              </button>
            )}
          </For>
        </div>
        <button class="pal-btn" onClick={() => props.onOpenPalette()}>
          <span>{t('shell.jump')}</span>
          <kbd>Ctrl K</kbd>
        </button>
      </div>
    </header>
  )
}
