import { expect, test } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const SNAPSHOT_PATH = resolve(__dirname, '../fixtures/tiny-ts-snapshot.json')

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
  await expect(page.locator('.strat-status')).toContainText(/\d+ modules/)
  await expect(page.locator('.strat-status')).toContainText(/\d+ edges/)
})
