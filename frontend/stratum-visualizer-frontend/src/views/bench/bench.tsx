import { Component, For, Show } from 'solid-js'
import { t } from '../../i18n'
import { ViewDef, ViewProps } from '../view-def'
import './bench.css'
import { attachModel, BenchResult, exposeBench, lastResult, RenderRun, runBench, TARGET, TaskResult } from './run'

exposeBench()

const fixed = (x: number | null | undefined, digits = 1): string => (x === null || x === undefined ? '—' : x.toFixed(digits))

const ratio = (x: number | null): string => (x === null ? '—' : `${(x * 100).toFixed(1)}%`)

const Verdict: Component<{ isMet: boolean }> = (props) => (
  <span class={`bn-badge ${props.isMet ? 'ok' : 'no'}`}>{props.isMet ? t('bench.met') : t('bench.miss')}</span>
)

const TaskRow: Component<{ task: TaskResult }> = (props) => (
  <tr data-testid="bench-task" data-id={props.task.id} data-status={props.task.status}>
    <td>{props.task.question}</td>
    <td>{props.task.who}</td>
    <Show
      when={props.task.status === 'run' ? props.task : null}
      fallback={
        <td colSpan={3} class="bn-dim">
          {props.task.status === 'skipped' ? t('bench.skipped', { reason: props.task.skip }) : ''}
        </td>
      }
    >
      {(run) => (
        <>
          <td class="mono">{run().answer}</td>
          <td class="num" title={run().actions.join('\n')}>
            {run().actions.length}
          </td>
          <td data-found={String(run().isAnswerShown)}>
            <Verdict isMet={run().isAnswerShown && run().actions.length <= TARGET.actions} />
          </td>
        </>
      )}
    </Show>
  </tr>
)

const RunRow: Component<{ run: RenderRun }> = (props) => {
  const r = props.run
  return (
    <tr data-testid="bench-run">
      <td>{r.view}</td>
      <td>{r.variant}</td>
      <td class="mono">{r.scenario}</td>
      <td class={`num${r.renderMs > TARGET.renderMs ? ' bn-fail' : ''}`}>{fixed(r.renderMs)}</td>
      <td class={`num${r.marks > TARGET.marks ? ' bn-fail' : ''}`}>{r.marks}</td>
      <td class="num">{r.edges}</td>
      <td class="num">{r.crossings ?? '—'}</td>
      <td class={`num${(r.panZoom?.p95 ?? 0) > TARGET.panZoomMs ? ' bn-fail' : ''}`}>{fixed(r.panZoom?.p95)}</td>
      <td class={`num${r.audit.leaks ? ' bn-fail' : ''}`} title={[...r.audit.samples, ...r.panelAudit.samples].join('\n')}>
        {r.audit.leaks} · {r.panelAudit.leaks}
      </td>
      <td class="num">{r.size ? r.size.join('×') : '—'}</td>
    </tr>
  )
}

const Results: Component<{ result: BenchResult }> = (props) => (
  <>
    <h3>{t('bench.tasks')}</h3>
    <table class="bn-table" data-testid="bench-tasks">
      <thead>
        <tr>
          <th>{t('bench.col.question')}</th>
          <th>{t('bench.col.who')}</th>
          <th>{t('bench.col.answer')}</th>
          <th>{t('bench.col.actions')}</th>
          <th>{t('bench.col.found')}</th>
        </tr>
      </thead>
      <tbody>
        <For each={props.result.tasks}>{(task) => <TaskRow task={task} />}</For>
      </tbody>
    </table>
    <h3>{t('bench.runs')}</h3>
    <table class="bn-table" data-testid="bench-runs">
      <thead>
        <tr>
          <th>{t('bench.col.view')}</th>
          <th>{t('bench.col.variant')}</th>
          <th>{t('bench.col.scenario')}</th>
          <th>{t('bench.col.render')}</th>
          <th>{t('bench.col.marks')}</th>
          <th>{t('bench.col.edges')}</th>
          <th>{t('bench.col.crossings')}</th>
          <th>{t('bench.col.panZoom')}</th>
          <th>{t('bench.col.leaks')}</th>
          <th>{t('bench.col.size')}</th>
        </tr>
      </thead>
      <tbody>
        <For each={props.result.runs}>{(run) => <RunRow run={run} />}</For>
      </tbody>
    </table>
    <h3>
      {t('bench.territory')} · {ratio(props.result.territory.precision)} · {ratio(props.result.territory.recall)}
    </h3>
    <table class="bn-table" data-testid="bench-territory">
      <thead>
        <tr>
          <th>{t('bench.col.rule')}</th>
          <th>{t('bench.col.agreed')}</th>
          <th>{t('bench.col.notLinted')}</th>
          <th>{t('bench.col.notShown')}</th>
        </tr>
      </thead>
      <tbody>
        <For each={props.result.territory.rules}>
          {(r) => (
            <tr>
              <td class="mono">{r.rule}</td>
              <td class="num">{r.shownAndLinted}</td>
              <td class={`num${r.shownNotLinted ? ' bn-fail' : ''}`}>{r.shownNotLinted}</td>
              <td class={`num${r.lintedNotShown ? ' bn-fail' : ''}`}>{r.lintedNotShown}</td>
            </tr>
          )}
        </For>
      </tbody>
    </table>
    <For each={props.result.territory.samples}>{(s) => <div class="mono bn-dim">{s}</div>}</For>
    <p class="bn-note">{t('bench.note')}</p>
  </>
)

const Stage: Component<ViewProps> = (props) => {
  attachModel(props.model)
  return (
    <div class="bench" data-testid="bench-stage">
      <div class="bn-bar">
        <button class="bn-btn" data-testid="bench-start" onClick={() => runBench()}>
          {lastResult() ? t('bench.rerun') : t('bench.run')}
        </button>
        <span class="bn-dim">
          {lastResult()
            ? `${lastResult()!.project} · ${lastResult()!.isPassing ? t('bench.passing') : t('bench.failing')}`
            : t('bench.idle')}
        </span>
      </div>
      <Show when={lastResult()}>{(result) => <Results result={result()} />}</Show>
    </div>
  )
}

const Details: Component<ViewProps> = () => (
  <aside class="panel panel-details">
    <header class="panel-hd">
      <span class="panel-title">{t('bench.targets')}</span>
    </header>
    <div class="panel-body">
      <Show when={lastResult()}>
        {(result) => (
          <div class="bn-targets" data-testid="bench-targets">
            <For each={result().targets}>
              {(target) => (
                <div class="bn-target" data-id={target.id} data-met={String(target.isMet)}>
                  <Verdict isMet={target.isMet} />
                  <span>
                    {t(`bench.target.${target.id}`)}
                    <br />
                    <span class="mono bn-dim">{target.value}</span>
                  </span>
                </div>
              )}
            </For>
          </div>
        )}
      </Show>
    </div>
  </aside>
)

const Outline: Component<ViewProps> = () => (
  <aside class="panel">
    <header class="panel-hd">
      <span class="panel-title">{t('view.bench')}</span>
    </header>
    <div class="panel-body">
      <p class="bn-note">{t('bench.idle')}</p>
    </div>
  </aside>
)

export const benchView: ViewDef = {
  id: 'bench',
  key: '9',
  title: 'view.bench',
  Stage,
  Outline,
  Details,
}
