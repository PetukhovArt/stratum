import { expect, Page, test } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const SNAPSHOT_PATH = fileURLToPath(new URL('../fixtures/tiny-ts-snapshot.json', import.meta.url))
const BRIEF = 'module src/shared/api/http.ts\n  may import: app, entities\n'

const LEGACY_TO_HTTP = {
  rule: 'stratum/no-cross-layer-import',
  severity: 'error',
  message: 'legacy may not import shared',
  file: 'src/legacy/old-helper.js',
  location: { line: 1, column: 1 },
  modules: [5, 6],
  edge: { from: 5, to: 6, kind: 'static' },
  suggestion: null,
}

const HTTP_PATH = 'src/shared/api/http.ts'
const INTERNALS_BEHIND_ENTRY = { rule: 'stratum/public-api', region: { kind: 'internals' } }

interface FixtureSnapshot {
  modules: { path: string; territory: number }[]
  territories: { may_not: unknown[] }[]
}

const fixtureSnapshot = (): FixtureSnapshot => JSON.parse(readFileSync(SNAPSHOT_PATH, 'utf-8'))

const withHttpForbiddenInternals = (snapshot: FixtureSnapshot): FixtureSnapshot => {
  const http = snapshot.modules.find((m) => m.path === HTTP_PATH)!
  const territory = { ...snapshot.territories[http.territory], may_not: [INTERNALS_BEHIND_ENTRY] }
  http.territory = snapshot.territories.push(territory) - 1
  return snapshot
}

const stubRoutes = async (page: Page, contextStatus: number, snapshot = fixtureSnapshot()) => {
  await page.route('**/api/snapshot', (route) =>
    route.fulfill({ body: JSON.stringify(snapshot), contentType: 'application/json' }),
  )
  await page.route('**/api/violations', (route) =>
    route.fulfill({ body: JSON.stringify([LEGACY_TO_HTTP]), contentType: 'application/json' }),
  )
  await page.route('**/api/context?**', (route) =>
    route.fulfill({ status: contextStatus, body: contextStatus === 200 ? BRIEF : 'not found', contentType: 'text/plain' }),
  )
}

test('focus opens on the most imported module with its importers and the lint finding on the drawn edge', async ({
  page,
}) => {
  await stubRoutes(page, 200, withHttpForbiddenInternals(fixtureSnapshot()))
  await page.goto('/')

  await expect(page.getByTestId('focus-card')).toContainText('http.ts', { timeout: 15_000 })
  await expect(page.locator('[data-testid="focus-column"][data-side="-1"] [data-testid="focus-row"]')).toHaveCount(2)
  await expect(page.locator('[data-edge][data-rule="stratum/no-cross-layer-import"]')).toHaveCount(1)
  await expect(page.getByTestId('territory-strip')).toContainText('internals behind another entry')
  await expect(page.getByTestId('focus-brief')).toHaveText(BRIEF)
})

test('h toggles the 2nd hop and a row click refocuses', async ({ page }) => {
  await stubRoutes(page, 404)
  await page.goto('/')
  await expect(page.getByTestId('focus-card')).toBeVisible({ timeout: 15_000 })
  await expect(page.locator('[data-testid="focus-column"]')).toHaveCount(2)

  await page.keyboard.press('h')
  await expect(page.locator('[data-testid="focus-column"]')).toHaveCount(4)
  await expect(page.getByTestId('focus-hops-2')).toHaveClass(/on/)

  await page.locator('[data-testid="focus-row"]', { hasText: 'old-helper.js' }).click()
  await expect(page.getByTestId('focus-card')).toContainText('old-helper.js')
  await expect(page.getByTestId('focus-brief')).toHaveCount(0)
})

test('the bands variant draws every declared layer', async ({ page }) => {
  await stubRoutes(page, 404)
  await page.goto('/')
  await expect(page.getByTestId('focus-card')).toBeVisible({ timeout: 15_000 })

  await page.getByRole('button', { name: 'Territory bands' }).click()
  await expect(page.getByTestId('focus-band')).toHaveCount(5)
})
