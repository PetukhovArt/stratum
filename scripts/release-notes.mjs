#!/usr/bin/env node
// Prints the body of the `## <version>` section of CHANGELOG.md: the GitHub release notes.
// Fails when the section is missing or empty, before anything is published.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const version = process.argv[2]
if (!version) {
  console.error('usage: node scripts/release-notes.mjs <version>')
  process.exit(2)
}
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const lines = fs.readFileSync(path.join(root, 'CHANGELOG.md'), 'utf8').split(/\r?\n/)
const start = lines.findIndex((line) => line === `## ${version}` || line.startsWith(`## ${version} `))
const end = start < 0 ? -1 : lines.findIndex((line, i) => i > start && line.startsWith('## '))
const body = start < 0 ? '' : lines.slice(start + 1, end < 0 ? undefined : end).join('\n').trim()
if (!body) {
  console.error(`CHANGELOG.md has no '## ${version}' section with entries`)
  process.exit(1)
}
console.log(body)
