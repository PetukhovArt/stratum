import { expect, test } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const SNAPSHOT_PATH = fileURLToPath(new URL('../fixtures/tiny-ts-snapshot.json', import.meta.url))

const stubRoutes = async (page: import('@playwright/test').Page) => {
  const snapshot = readFileSync(SNAPSHOT_PATH, 'utf-8')
  await page.route('**/api/snapshot', (route) => {
    route.fulfill({ body: snapshot, contentType: 'application/json' })
  })
  await page.route('**/api/violations', (route) => {
    route.fulfill({ body: '[]', contentType: 'application/json' })
  })
}

test('container map packs units into layer lanes and expands one into modules', async ({ page }) => {
  await stubRoutes(page)
  await page.goto('/')
  await expect(page.locator('.app')).toBeVisible({ timeout: 15_000 })
  await page.keyboard.press('4')

  const map = page.locator('svg.cv g.map[data-variant="lanes"]')
  await expect(map).toBeVisible()
  await expect(map.getByTestId('map-unit')).toHaveCount(5)

  await page.getByTestId('map-gate-all').click()
  await expect(page.getByTestId('map-gate-all')).toHaveText('all 6')
  await expect(map.locator('path[data-edge^="unit:"]')).toHaveCount(6)

  const shared = map.locator('[data-testid="map-unit"][data-id="4"]')
  await shared.dblclick()
  await expect(map.getByTestId('map-cell')).toHaveCount(3)
  await map.locator('[data-testid="map-cell"][data-id="8"]').click()
  await expect(page.getByTestId('focus-stage')).toBeVisible()

  await page.keyboard.press('4')
  await page.keyboard.press('v')
  await expect(page.locator('svg.cv g.map[data-variant="layers"]').getByTestId('map-layer')).toHaveCount(5)
})

test('status bar surfaces module + edge counts', async ({ page }) => {
  await stubRoutes(page)
  await page.goto('/')
  await expect(page.getByTestId('status-graph')).toHaveText(/^9 modules · 8 edges · 0 violations$/, {
    timeout: 15_000,
  })
})
