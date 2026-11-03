#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'

const [base = 'main', next = 'pr'] = process.argv.slice(2)
const threshold = Number(process.env.BENCH_THRESHOLD_PCT ?? 10)
const baselineDir = path.join('target', 'criterion')

if (!fs.existsSync(baselineDir)) {
  console.log(`::warning::no criterion baseline dir at ${baselineDir} — skipping compare`)
  process.exit(0)
}

const mean = (file) => JSON.parse(fs.readFileSync(file, 'utf8')).mean.point_estimate

let regressions = 0
for (const baseJson of findEstimates(baselineDir, base)) {
  const benchDir = path.dirname(path.dirname(baseJson))
  const benchName = path.basename(benchDir)
  const newJson = path.join(benchDir, next, 'estimates.json')
  if (!fs.existsSync(newJson)) {
    console.log(`skip: ${benchName} (no ${next} baseline)`)
    continue
  }
  const b = mean(baseJson)
  const n = mean(newJson)
  const delta = ((n - b) / b) * 100
  const sign = delta >= 0 ? '+' : ''
  console.log(`${benchName.padEnd(60)} base=${b.toFixed(2)}ns new=${n.toFixed(2)}ns delta=${sign}${delta.toFixed(2)}%`)
  if (delta > threshold) {
    console.log(`::error::${benchName} regressed by ${delta.toFixed(2)}% (threshold ${threshold}%)`)
    regressions++
  }
}

if (regressions > 0) {
  console.log(`${regressions} bench(es) regressed beyond ${threshold}%`)
  process.exit(1)
}
console.log('no regressions')

function findEstimates(dir, baseline) {
  const found = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const entryPath = path.join(dir, entry.name)
    if (!entry.isDirectory()) continue
    const estimates = path.join(entryPath, 'estimates.json')
    if (entry.name === baseline && fs.existsSync(estimates)) found.push(estimates)
    else found.push(...findEstimates(entryPath, baseline))
  }
  return found
}
