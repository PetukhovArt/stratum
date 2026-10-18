import { expect, Page, test } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const SNAPSHOT_PATH = fileURLToPath(new URL('../fixtures/tiny-ts-snapshot.json', import.meta.url))
const BENCH_URL = process.env.BENCH_URL
const BENCH_TIMEOUT_MS = 300_000

const CROSS_LAYER = 'stratum/no-cross-layer-import'
const LINT_EDGE_FINDINGS: readonly [rule: string, from: string, to: string][] = [
  [CROSS_LAYER, 'src/app/main.ts', 'src/features/cart/index.ts'],
  [CROSS_LAYER, 'src/app/main.ts', 'src/shared/api/http.ts'],
  [CROSS_LAYER, 'src/entities/user/User.ts', 'src/shared/utils/format.ts'],
  [CROSS_LAYER, 'src/features/cart/cart.tsx', 'src/entities/user/index.ts'],
  [CROSS_LAYER, 'src/features/cart/cart.tsx', 'src/shared/ui/Button.tsx'],
  [CROSS_LAYER, 'src/legacy/old-helper.js', 'src/shared/api/http.ts'],
]

const tinyTsViolations = (snapshot: { modules: { id: number; path: string }[] }) => {
  const idOf = new Map(snapshot.modules.map((m) => [m.path, m.id]))
  return LINT_EDGE_FINDINGS.map(([rule, from, to]) => ({
    rule,
    severity: 'warning',
    message: 'cross-layer import',
    file: from,
    location: { line: 1, column: 1 },
    modules: [idOf.get(from), idOf.get(to)],
    edge: { from: idOf.get(from), to: idOf.get(to), kind: 'static' },
    fix: null,
  }))
}

const TINY_TS_DRIFT = {
  base: { ref: 'HEAD~1', sha: 'abc1234' },
  markdown: '<!-- stratum:drift -->\n### Stratum: architecture drift vs abc1234\n1 import that breaks rules\n',
  breaking: [0],
  edges: [
    {
      from: 'src/legacy/old-helper.js',
      to: 'src/shared/ui/Button.tsx',
      kind: 'static',
      change: 'added',
      breaks: [[CROSS_LAYER, 'error']],
    },
    { from: 'src/features/cart/cart.tsx', to: 'src/features/cart/gone.ts', kind: 'static', change: 'removed', breaks: [] },
  ],
  layer_edges: [],
  slice_edges: [],
  cycles: [],
  stages: [],
  fan_in: [],
}

const serveTinyTs = async (page: Page) => {
  const body = readFileSync(SNAPSHOT_PATH, 'utf-8')
  await page.route('**/api/snapshot', (route) => route.fulfill({ body, contentType: 'application/json' }))
  await page.route('**/api/violations', (route) =>
    route.fulfill({ body: JSON.stringify(tinyTsViolations(JSON.parse(body))), contentType: 'application/json' }),
  )
  await page.route('**/api/context?**', (route) => route.fulfill({ status: 404, body: 'not found' }))
  await page.route('**/api/drift', (route) =>
    route.fulfill({ body: JSON.stringify(TINY_TS_DRIFT), contentType: 'application/json' }),
  )
}

test('the bench answers every phase-1 question and meets every gate', async ({ page }) => {
  test.setTimeout(BENCH_TIMEOUT_MS)
  if (!BENCH_URL) await serveTinyTs(page)
  await page.goto(BENCH_URL ?? '/')
  await page.waitForFunction(() => window.__stratumBench !== undefined, null, {
    timeout: 60_000,
  })
  await page.waitForLoadState('networkidle')

  const result = await page.evaluate(() => window.__stratumBench!.run())
  console.log(JSON.stringify(result, null, 2))

  const ran = result.tasks.filter((t) => t.status === 'run')
  const isMrShown = result.tasks.some((t) => t.id === 'mr-breaks' && t.status === 'run')
  if (!BENCH_URL) expect(isMrShown).toBe(true)
  expect(ran.map((t) => t.id).sort()).toEqual(
    [
      'cycle-cut',
      'fan-in-hotspot',
      'importers-in-layer',
      'layer-pairs-against',
      'may-import',
      'may-not-import',
      'unit-cycles',
      ...(isMrShown ? ['mr-breaks'] : []),
    ].sort(),
  )
  expect(result.runs.filter((r) => r.view === 'mr').map((r) => r.variant)).toEqual(isMrShown ? ['region', 'card'] : [])
  expect(ran.filter((t) => !t.isAnswerShown || t.actions.length > 3)).toEqual([])
  expect(result.runs.length).toBeGreaterThan(2)
  expect(result.targets.filter((t) => t.isGate && !t.isMet)).toEqual([])
  if (!BENCH_URL) expect(result.territory.rules.map((r) => r.rule)).toEqual([CROSS_LAYER])
  await expect(page.getByTestId('bench-runs').getByTestId('bench-run')).toHaveCount(result.runs.length)
})
