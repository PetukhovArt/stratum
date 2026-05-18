import { expect, test } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const SNAPSHOT_PATH = resolve(__dirname, '../fixtures/tiny-ts-snapshot.json')

test('visualizer mounts a canvas backed by WebGL', async ({ page }) => {
  const snapshot = readFileSync(SNAPSHOT_PATH, 'utf-8')

  await page.route('**/api/snapshot', (route) => {
    route.fulfill({ body: snapshot, contentType: 'application/json' })
  })

  await page.goto('/')
  const canvas = page.locator('canvas')
  await expect(canvas).toBeVisible({ timeout: 15_000 })

  const hasGL = await page.evaluate(() => {
    const cv = document.querySelector('canvas') as HTMLCanvasElement | null
    if (!cv) return false
    const gl = cv.getContext('webgl2') || cv.getContext('webgl')
    return !!gl
  })
  expect(hasGL).toBe(true)
})
