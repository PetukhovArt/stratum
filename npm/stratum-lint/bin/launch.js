'use strict'

const path = require('path')
const { spawnSync } = require('child_process')
const { name: launcher, optionalDependencies } = require('../package.json')

module.exports = (binary) => {
  const platformPackage = `${launcher}-${process.platform}-${process.arch}`
  if (!(platformPackage in optionalDependencies)) {
    fail(`${binary}: no prebuilt binary for ${process.platform}-${process.arch}. Supported: ${Object.keys(optionalDependencies).join(', ')}`)
  }
  let packageDir
  try {
    packageDir = path.dirname(require.resolve(`${platformPackage}/package.json`))
  } catch {
    fail(`${binary}: ${platformPackage} is not installed. npm skips optional dependencies with --omit=optional / --no-optional, or when the lockfile was written on another platform. Reinstall ${launcher} without those, or install ${platformPackage}@${optionalDependencies[platformPackage]} explicitly.`)
  }
  const executable = path.join(packageDir, 'bin', process.platform === 'win32' ? `${binary}.exe` : binary)
  const result = spawnSync(executable, process.argv.slice(2), { stdio: 'inherit', windowsHide: true })
  if (result.error) {
    fail(`${binary}: cannot run ${executable}: ${result.error.message}`)
  }
  if (result.signal) {
    process.kill(process.pid, result.signal)
  }
  process.exit(result.status ?? 1)
}

function fail(message) {
  console.error(message)
  process.exit(2)
}
