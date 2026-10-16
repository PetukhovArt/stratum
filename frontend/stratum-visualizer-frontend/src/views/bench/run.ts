import { createSignal } from 'solid-js'
import { Model } from '../../render/model'
import { NavHooks, NavPatch } from '../../shell/nav'
import { layoutBands } from '../focus/bands'
import { layoutColumns } from '../focus/columns'
import {
  auditChannel,
  boundsSize,
  ChannelAudit,
  countCrossings,
  countMarks,
  forceLayout,
  lineEdges,
  measurePanZoom,
  PanZoom,
  severityColours,
  stageSvg,
} from './probes'
import { pickSubjects, Subject, Subjects } from './subjects'
import { Asker, PENDING_TASKS, PendingTask, TASK_BLOCKS } from './tasks'
import { drawnPairsOf, DrawnPairs, TerritoryAgreement, TerritoryTally } from './territory'

export const TARGET = { renderMs: 100, marks: 1500, panZoomMs: 16.7, actions: 3 } as const

const STAGE_REGION = '.stage-region'
const PANELS = '.panel-slot'

export interface RenderRun {
  view: 'focus' | 'dsm'
  variant: string
  scenario: string
  renderMs: number
  marks: number
  edges: number
  crossings: number | null
  panZoom: PanZoom | null
  audit: ChannelAudit
  panelAudit: ChannelAudit
  size: [w: number, h: number] | null
}

export type TaskResult =
  | {
      status: 'run'
      order: number
      id: string
      question: string
      who: Asker
      view: string
      answer: string
      actions: string[]
      isAnswerShown: boolean
      ms: number
    }
  | ({ status: 'pending' } & PendingTask)

export type TargetId = 'tasks' | 'marks' | 'render' | 'pan-zoom' | 'leaks' | 'precision' | 'recall'

export interface TargetResult {
  id: TargetId
  value: string
  isMet: boolean
  isGate: boolean
}

export interface BenchResult {
  project: string
  subjects: Subject[]
  runs: RenderRun[]
  tasks: TaskResult[]
  territory: TerritoryAgreement
  targets: TargetResult[]
  isPassing: boolean
}

interface Scenario {
  view: RenderRun['view']
  variant: string
  scenario: string
  patch: NavPatch
  drawn?: { scene: string; pairs: () => DrawnPairs }
}

declare global {
  interface Window {
    __benchRun?: () => BenchResult
    __bench?: BenchResult
  }
}

const [lastResult, setLastResult] = createSignal<BenchResult | null>(null)
export { lastResult }

let benchModel: Model | null = null

export const attachModel = (m: Model): void => {
  benchModel = m
}

const navHooks = (): NavHooks => {
  const hooks = window.__stratum
  if (!hooks) throw new Error('The bench needs window.__stratum: the app has not mounted yet')
  return hooks
}

const NO_EXPANSION = new Map<string, boolean>()

const focusScenarios = (m: Model, s: Subjects): Scenario[] =>
  s.list.flatMap((subject): Scenario[] => {
    const base = { view: 'focus' as const, scenario: `${subject.role}: ${subject.path}` }
    const focus = { view: 'focus' as const, focus: subject.id }
    const columns = { scene: `columns:${subject.id}` }
    return [
      {
        ...base,
        variant: 'columns, 1 hop',
        patch: { ...focus, focusVariant: 'columns', hops: 1 },
        drawn: { ...columns, pairs: () => drawnPairsOf(layoutColumns(m, subject.id, 1, NO_EXPANSION).bundles) },
      },
      {
        ...base,
        variant: 'columns, 2 hops',
        patch: { ...focus, focusVariant: 'columns', hops: 2 },
        drawn: { ...columns, pairs: () => drawnPairsOf(layoutColumns(m, subject.id, 2, NO_EXPANSION).bundles) },
      },
      {
        ...base,
        variant: 'bands',
        patch: { ...focus, focusVariant: 'bands' },
        drawn: {
          scene: `bands:${subject.id}`,
          pairs: () => drawnPairsOf(layoutBands(m, subject.id).links.map((l) => ({ key: l.key, pairs: l.pill.pairs }))),
        },
      },
    ]
  })

const DSM_SCENARIOS: readonly Scenario[] = [
  { view: 'dsm', variant: 'partitioned', scenario: 'layer level', patch: { view: 'dsm', dsmLevel: 'layer' } },
  { view: 'dsm', variant: 'partitioned', scenario: 'unit level', patch: { view: 'dsm', dsmLevel: 'unit' } },
]

const auditPanels = (colours: ReadonlySet<string>): ChannelAudit => {
  const audits = [...document.querySelectorAll(PANELS)].map((panel) => auditChannel(panel, colours))
  return {
    checked: audits.reduce((n, a) => n + a.checked, 0),
    leaks: audits.reduce((n, a) => n + a.leaks, 0),
    samples: audits.flatMap((a) => a.samples),
  }
}

const measureRun = (hooks: NavHooks, sc: Scenario, colours: ReadonlySet<string>, tally: TerritoryTally): RenderRun => {
  hooks.go({ view: 'bench' })
  forceLayout()
  const t0 = performance.now()
  hooks.go(sc.patch)
  forceLayout()
  const renderMs = performance.now() - t0
  const svg = stageSvg()
  const edges = lineEdges(svg)
  const stage = document.querySelector(STAGE_REGION)
  if (sc.drawn) tally.record(sc.drawn.scene, sc.drawn.pairs(), stage)
  return {
    view: sc.view,
    variant: sc.variant,
    scenario: sc.scenario,
    renderMs,
    marks: countMarks(svg),
    edges: edges.length,
    crossings: countCrossings(edges),
    panZoom: measurePanZoom(),
    audit: auditChannel(stage, colours),
    panelAudit: auditPanels(colours),
    size: boundsSize(svg),
  }
}

const runTasks = (hooks: NavHooks, m: Model, s: Subjects): TaskResult[] => {
  const ran = TASK_BLOCKS.flatMap((block) => block(m, s)).map((task): TaskResult => {
    hooks.go({ view: 'bench' })
    const t0 = performance.now()
    for (const step of task.steps) hooks.go(step.patch)
    forceLayout()
    const { steps, isAnswerShown, ...rest } = task
    return {
      ...rest,
      status: 'run',
      actions: steps.map((st) => st.label),
      isAnswerShown: isAnswerShown(),
      ms: performance.now() - t0,
    }
  })
  const pending = PENDING_TASKS.map((p): TaskResult => ({ ...p, status: 'pending' }))
  return [...ran, ...pending].sort((a, b) => a.order - b.order)
}

const percent = (x: number | null): string => (x === null ? 'n/a' : `${(x * 100).toFixed(1)}%`)

const worstOf = <T>(items: readonly T[], score: (x: T) => number): T | undefined =>
  items.reduce<T | undefined>((best, x) => (best === undefined || score(x) > score(best) ? x : best), undefined)

const runLabel = (r: RenderRun): string => `${r.view} ${r.variant} · ${r.scenario}`

const judge = (runs: readonly RenderRun[], tasks: readonly TaskResult[], territory: TerritoryAgreement): TargetResult[] => {
  const ran = tasks.filter((t) => t.status === 'run')
  const answered = ran.filter((t) => t.isAnswerShown && t.actions.length <= TARGET.actions)
  const byMarks = worstOf(runs, (r) => r.marks)
  const byRender = worstOf(runs, (r) => r.renderMs)
  const byPanZoom = worstOf(runs, (r) => r.panZoom?.p95 ?? 0)
  const leaks = runs.reduce((n, r) => n + r.audit.leaks + r.panelAudit.leaks, 0)
  return [
    { id: 'tasks', value: `${answered.length}/${ran.length}`, isMet: answered.length === ran.length, isGate: true },
    {
      id: 'marks',
      value: byMarks ? `${byMarks.marks} · ${runLabel(byMarks)}` : 'n/a',
      isMet: (byMarks?.marks ?? 0) <= TARGET.marks,
      isGate: true,
    },
    {
      id: 'render',
      value: byRender ? `${byRender.renderMs.toFixed(1)} ms · ${runLabel(byRender)}` : 'n/a',
      isMet: (byRender?.renderMs ?? 0) <= TARGET.renderMs,
      isGate: false,
    },
    {
      id: 'pan-zoom',
      value: byPanZoom?.panZoom ? `${byPanZoom.panZoom.p95.toFixed(1)} ms · ${runLabel(byPanZoom)}` : 'n/a',
      isMet: (byPanZoom?.panZoom?.p95 ?? 0) <= TARGET.panZoomMs,
      isGate: true,
    },
    { id: 'leaks', value: String(leaks), isMet: leaks === 0, isGate: true },
    { id: 'precision', value: percent(territory.precision), isMet: (territory.precision ?? 1) === 1, isGate: true },
    { id: 'recall', value: percent(territory.recall), isMet: (territory.recall ?? 1) === 1, isGate: true },
  ]
}

export const runBench = (): BenchResult => {
  const hooks = navHooks()
  const keep = hooks.get()
  hooks.go({ view: 'bench' })
  const m = benchModel
  if (!m) throw new Error('The bench view did not mount')
  const subjects = pickSubjects(m)
  const colours = severityColours()
  const tally = new TerritoryTally(m.violations)
  const tasks = runTasks(hooks, m, subjects)
  const runs = [...focusScenarios(m, subjects), ...DSM_SCENARIOS].map((sc) => measureRun(hooks, sc, colours, tally))
  const territory = tally.result()
  const targets = judge(runs, tasks, territory)
  const result: BenchResult = {
    project: m.project,
    subjects: [...subjects.list],
    runs,
    tasks,
    territory,
    targets,
    isPassing: targets.every((t) => t.isMet || !t.isGate),
  }
  hooks.go({ ...keep, view: 'bench' })
  window.__bench = result
  setLastResult(result)
  return result
}

export const exposeBench = (): void => {
  window.__benchRun = runBench
}
