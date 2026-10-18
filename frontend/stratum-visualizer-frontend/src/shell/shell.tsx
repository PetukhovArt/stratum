import { Component, createMemo, createResource, createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { Dynamic } from 'solid-js/web'
import './fonts.css'
import './shell.css'
import '../styles.css'
import { t } from '../i18n'
import { loadSnapshot, SnapshotVersionMismatchError } from '../lib/snapshot'
import { loadViolations } from '../lib/violations'
import { buildModel, Model } from '../render/model'
import { viewById, ViewDef } from '../views'
import { Header } from './header'
import { bindKeymap } from './keymap'
import { nav, set } from './nav'
import { Palette } from './palette'
import { StatusBar } from './status-bar'
import { variantIndex, variantPatch } from './variants'

const PANEL = {
  leftWidth: 264,
  rightWidth: 340,
  minWidth: 200,
  maxWidth: 560,
  handleWidth: '5px',
} as const

export const Shell: Component = () => {
  const [snapshot] = createResource(() => loadSnapshot('/api/snapshot'))
  const [violations] = createResource(() => loadViolations('/api/violations'))
  const isViolationsUnavailable = () => violations.state === 'errored'

  const model = createMemo<Model | null>(() => {
    const s = snapshot.state === 'ready' ? snapshot() : undefined
    const v = isViolationsUnavailable() ? [] : violations.state === 'ready' ? violations() : undefined
    if (!s || !v) return null
    return buildModel(s, v)
  })

  return (
    <Show when={!snapshot.error} fallback={<ErrorView err={snapshot.error as Error} />}>
      <Show when={model()} fallback={<div class="loading">{t('shell.loading')}</div>}>
        {(m) => <Layout model={m()} isViolationsUnavailable={isViolationsUnavailable()} />}
      </Show>
    </Show>
  )
}

const Layout: Component<{ model: Model; isViolationsUnavailable: boolean }> = (props) => {
  const [isPaletteOpen, setPaletteOpen] = createSignal(false)
  const [isLeftCollapsed, setLeftCollapsed] = createSignal(false)
  const [isRightCollapsed, setRightCollapsed] = createSignal(false)
  const [leftWidth, setLeftWidth] = createSignal<number>(PANEL.leftWidth)
  const [rightWidth, setRightWidth] = createSignal<number>(PANEL.rightWidth)
  const view = createMemo(() => viewById(nav().view))

  onMount(() => {
    const unbind = bindKeymap({
      isPaletteOpen,
      setPaletteOpen,
      toggleLeftPanel: () => setLeftCollapsed(!isLeftCollapsed()),
      toggleRightPanel: () => setRightCollapsed(!isRightCollapsed()),
    })
    onCleanup(unbind)
  })

  const bodyStyle = () => {
    const column = (isCollapsed: boolean, width: number) => (isCollapsed ? '0' : `${width}px`)
    const handle = (isCollapsed: boolean) => (isCollapsed ? '0' : PANEL.handleWidth)
    return {
      'grid-template-columns': [
        column(isLeftCollapsed(), leftWidth()),
        handle(isLeftCollapsed()),
        'minmax(0, 1fr)',
        handle(isRightCollapsed()),
        column(isRightCollapsed(), rightWidth()),
      ].join(' '),
    }
  }

  const viewProps = () => ({ model: props.model, isViolationsUnavailable: props.isViolationsUnavailable })

  return (
    <div class="app">
      <Header model={props.model} onOpenPalette={() => setPaletteOpen(true)} />
      <div class="body" style={bodyStyle()}>
        <div class="panel-slot">
          <Show when={!isLeftCollapsed()}>
            <Dynamic component={view().Outline} {...viewProps()} />
          </Show>
        </div>
        <Show when={!isLeftCollapsed()} fallback={<div />}>
          <ResizeHandle side="left" width={leftWidth} setWidth={setLeftWidth} />
        </Show>
        <div class="stage-wrap">
          <VariantBar view={view()} />
          <div class="stage-region">
            <Dynamic component={view().Stage} {...viewProps()} />
          </div>
        </div>
        <Show when={!isRightCollapsed()} fallback={<div />}>
          <ResizeHandle side="right" width={rightWidth} setWidth={setRightWidth} />
        </Show>
        <div class="panel-slot">
          <Show when={!isRightCollapsed()}>
            <Dynamic component={view().Details} {...viewProps()} />
          </Show>
        </div>
      </div>
      <StatusBar model={props.model} />
      <Show when={isPaletteOpen()}>
        <Palette model={props.model} onClose={() => setPaletteOpen(false)} />
      </Show>
    </div>
  )
}

const VariantBar: Component<{ view: ViewDef }> = (props) => (
  <Show when={props.view.variants}>
    {(variants) => (
      <div class="varbar">
        <span class="lbl">{t('shell.variant')}</span>
        <For each={variants().options}>
          {(option, i) => (
            <button
              class={variantIndex(variants(), nav()) === i() ? 'on' : ''}
              onClick={() => set(variantPatch(variants(), i()))}
            >
              {t(option.title)}
            </button>
          )}
        </For>
        <kbd>v</kbd>
      </div>
    )}
  </Show>
)

const ResizeHandle: Component<{
  side: 'left' | 'right'
  width: () => number
  setWidth: (n: number) => void
}> = (props) => {
  let stopDrag: (() => void) | null = null
  onCleanup(() => stopDrag?.())

  const onMouseDown = (e: MouseEvent) => {
    e.preventDefault()
    const startX = e.clientX
    const startW = props.width()
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX
      const next = props.side === 'left' ? startW + dx : startW - dx
      props.setWidth(Math.max(PANEL.minWidth, Math.min(PANEL.maxWidth, next)))
    }
    const onUp = () => {
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
      stopDrag = null
    }
    stopDrag = onUp
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', onUp)
  }
  return (
    <div class={`resize-handle handle-${props.side}`} onMouseDown={onMouseDown}>
      <div class="resize-handle-grip" />
    </div>
  )
}

const ErrorView: Component<{ err: Error }> = (props) => {
  const isVersionMismatch = props.err instanceof SnapshotVersionMismatchError
  return (
    <div role="alert" class="err-view">
      <strong>{isVersionMismatch ? t('shell.outOfDate') : t('shell.snapshotError')}</strong>
      <p>{props.err.message}</p>
    </div>
  )
}
