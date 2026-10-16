import { expect, Page, test } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const SNAPSHOT_PATH = fileURLToPath(new URL('../fixtures/tiny-ts-snapshot.json', import.meta.url))
const MARKDOWN = '<!-- stratum:drift -->\n### Stratum: architecture drift vs abc1234\n1 import that breaks rules\n'

const DRIFT = {
  base: { ref: 'HEAD~1', sha: 'abc1234' },
  markdown: MARKDOWN,
  breaking: [0],
  edges: [
    {
      from: 'src/legacy/old-helper.js',
      to: 'src/shared/ui/Button.tsx',
      kind: 'static',
      change: 'added',
      breaks: [['stratum/no-cross-layer-import', 'error']],
    },
    { from: 'src/features/cart/cart.tsx', to: 'src/features/cart/gone.ts', kind: 'static', change: 'removed', breaks: [] },
  ],
  layer_edges: [],
  slice_edges: [],
  cycles: [],
  stages: [],
  fan_in: [],
}

const stubRoutes = async (page: Page, hasDrift: boolean) => {
  await page.route('**/api/snapshot', (route) =>
    route.fulfill({ body: readFileSync(SNAPSHOT_PATH, 'utf-8'), contentType: 'application/json' }),
  )
  await page.route('**/api/violations', (route) => route.fulfill({ body: '[]', contentType: 'application/json' }))
  await page.route('**/api/context?**', (route) => route.fulfill({ status: 404, body: 'not found' }))
  await page.route('**/api/drift', (route) =>
    hasDrift
      ? route.fulfill({ body: JSON.stringify(DRIFT), contentType: 'application/json' })
      : route.fulfill({ status: 404, body: 'no drift', contentType: 'text/plain' }),
  )
}

test('the merge request region tags the added rule break and the removed import of a deleted file', async ({ page }) => {
  await stubRoutes(page, true)
  await page.goto('/')
  await page.getByTestId('tab-mr').click({ timeout: 15_000 })

  const added = page.locator('[data-edge][data-tag="added"][data-rule="stratum/no-cross-layer-import"]')
  await expect(added).toHaveCount(1)
  await expect(page.locator('[data-edge][data-tag="removed"]')).toHaveCount(1)
  await expect(page.locator('[data-testid="mr-node"][data-ghost]')).toContainText('cart/gone.ts')

  await page.getByTestId('mr-file').filter({ hasText: 'old-helper.js' }).click()
  await expect(page.getByTestId('focus-card')).toContainText('old-helper.js')
})

test('the comment card shows the drift markdown verbatim and fits 800×600', async ({ page }) => {
  await stubRoutes(page, true)
  await page.goto('/')
  await page.getByTestId('tab-mr').click({ timeout: 15_000 })
  await page.getByRole('button', { name: 'Comment card' }).click()

  await expect(page.getByTestId('mr-markdown')).toHaveText(MARKDOWN, { useInnerText: true })
  const [w, h] = ((await page.getByTestId('mr-card').getAttribute('data-size')) ?? '').split('x').map(Number)
  expect(w).toBeLessThanOrEqual(800)
  expect(h).toBeLessThanOrEqual(600)
})

test('without --base the merge request tab is hidden', async ({ page }) => {
  await stubRoutes(page, false)
  await page.goto('/')
  await expect(page.getByTestId('tab-dsm')).toBeVisible({ timeout: 15_000 })
  await expect(page.getByTestId('tab-mr')).toHaveCount(0)
})
