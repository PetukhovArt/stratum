'use strict'

const path = require('path')
const { spawnSync } = require('child_process')

module.exports = (name) => {
  const ext = process.platform === 'win32' ? '.exe' : ''
  const bin = path.join(__dirname, '..', 'binary', `${name}${ext}`)
  const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' })
  if (result.error) {
    console.error(`[${name}] ${result.error.message}`)
  }
  process.exit(result.status ?? 1)
}
