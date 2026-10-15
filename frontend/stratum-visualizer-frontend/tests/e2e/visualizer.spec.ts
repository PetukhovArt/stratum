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

  await expect(page.locator('.strat-app')).toBeVisible({ timeout: 15_000 })
  await expect(page.locator('.strat-hd')).toBeVisible()

  const svg = page.locator('svg.strat-graph')
  await expect(svg).toBeVisible()
  await expect(svg.locator('g.containers')).toBeVisible()
  await expect(svg.locator('g.modules')).toBeVisible()
})

test('status bar surfaces module + edge counts', async ({ page }) => {
  await stubRoutes(page)
  await page.goto('/')
  await expect(page.locator('.strat-status')).toBeVisible({ timeout: 15_000 })
  await expect(page.locator('.strat-status .status-l .status-info')).toHaveText(
    /^9 modules · 5 containers · 8 edges$/,
  )
})
