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

test('visualizer mounts SVG graph with lanes, containers, and modules', async ({ page }) => {
  await stubRoutes(page)
  await page.goto('/')

  await expect(page.locator('.app')).toBeVisible({ timeout: 15_000 })
  await expect(page.locator('header.hd')).toBeVisible()

  const graph = page.locator('svg.cv g.strat-graph')
  await expect(graph).toBeVisible()
  await expect(graph.locator('g.containers')).toBeVisible()
  await expect(graph.locator('g.modules')).toBeVisible()
})

test('status bar surfaces module + edge counts', async ({ page }) => {
  await stubRoutes(page)
  await page.goto('/')
  await expect(page.getByTestId('status-graph')).toHaveText(/^9 modules · 8 edges · 0 violations$/, {
    timeout: 15_000,
  })
})
