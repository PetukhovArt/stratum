#!/usr/bin/env node
// Fails unless every place that carries the release version agrees: each crate, both Claude
// plugins and the npm pins in the CI templates. `--print` prints the version instead.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const read = (rel) => fs.readFileSync(path.join(root, rel), 'utf8')

const found = []
for (const crate of fs.readdirSync(path.join(root, 'crates'))) {
  const manifest = `crates/${crate}/Cargo.toml`
  const match = read(manifest).match(/^version\s*=\s*"([^"]+)"/m)
  found.push([manifest, match?.[1]])
}
for (const plugin of ['claude-plugin', 'claude-plugin-lsp']) {
  const manifest = `${plugin}/.claude-plugin/plugin.json`
  found.push([manifest, JSON.parse(read(manifest)).version])
}
for (const template of ['ci/github/action.yml', 'ci/gitlab/templates/stratum.yml']) {
  found.push([template, read(template).match(/@petukhovart\/stratum-lint@([^"\s]+)/)?.[1]])
}

const version = found[found.findIndex(([file]) => file === 'crates/stratum-lint/Cargo.toml')][1]
const wrong = found.filter(([, v]) => v !== version)
if (wrong.length > 0) {
  for (const [file, v] of wrong) {
    console.error(`${file}: ${v ?? 'no version'}, crates/stratum-lint is ${version}`)
  }
  process.exit(1)
}
console.log(process.argv.includes('--print') ? version : `version ${version} in ${found.length} places`)
