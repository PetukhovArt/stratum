#!/usr/bin/env node
// Builds the npm tarballs from the release archives: one platform package per
// `stratum-lint-<target>.{tar.gz,zip}`, then the launcher from npm/stratum-lint pinned to them.
// Usage: node scripts/npm-pack.mjs <archives-dir> <version> <out-dir>
// Prints the tarball paths in publish order: platform packages first, launcher last.

import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const launcherSource = path.join(repoRoot, 'npm', 'stratum-lint')

const NPM_PLATFORMS = {
  'x86_64-pc-windows-msvc': { os: 'win32', cpu: 'x64' },
  'aarch64-apple-darwin': { os: 'darwin', cpu: 'arm64' },
  'x86_64-unknown-linux-gnu': { os: 'linux', cpu: 'x64', libc: 'glibc' },
}

const [archivesArg, version, outArg] = process.argv.slice(2)
if (!archivesArg || !version || !outArg) {
  die('usage: node scripts/npm-pack.mjs <archives-dir> <version> <out-dir>')
}
const archivesDir = path.resolve(archivesArg)
const outDir = path.resolve(outArg)
fs.mkdirSync(outDir, { recursive: true })
const launcherManifest = readJson(path.join(launcherSource, 'package.json'))
const binaries = Object.keys(launcherManifest.bin)

const workDir = path.join(outDir, '.work')
fs.rmSync(workDir, { recursive: true, force: true })
fs.mkdirSync(workDir, { recursive: true })

const platformTarballs = []
const optionalDependencies = {}
for (const [triple, archive] of archivesByTarget()) {
  const platform = NPM_PLATFORMS[triple] ?? die(`no npm os/cpu mapping for target ${triple}; add it to NPM_PLATFORMS`)
  const name = `${launcherManifest.name}-${platform.os}-${platform.cpu}`
  if (name in optionalDependencies) {
    die(`targets map to the same npm package ${name}`)
  }
  const packageDir = path.join(workDir, name.replace('/', '__'))
  fs.mkdirSync(path.join(packageDir, 'bin'), { recursive: true })
  const ext = platform.os === 'win32' ? '.exe' : ''
  copyExecutables(archive, binaries.map((binary) => binary + ext), packageDir)
  writeJson(path.join(packageDir, 'package.json'), {
    name,
    version,
    description: `The ${platform.os}-${platform.cpu} binaries of ${launcherManifest.name}.`,
    license: launcherManifest.license,
    repository: launcherManifest.repository,
    homepage: launcherManifest.homepage,
    os: [platform.os],
    cpu: [platform.cpu],
    ...(platform.libc && { libc: [platform.libc] }),
    files: ['bin'],
    preferUnplugged: true,
  })
  fs.writeFileSync(
    path.join(packageDir, 'README.md'),
    `# \`${name}\`\n\nThe ${triple} binaries of [\`${launcherManifest.name}\`](https://www.npmjs.com/package/${launcherManifest.name}). Install that package instead.\n`,
  )
  copyLicenses(packageDir)
  optionalDependencies[name] = version
  platformTarballs.push(pack(packageDir))
}

const launcherDir = path.join(workDir, 'launcher')
fs.cpSync(launcherSource, launcherDir, { recursive: true })
writeJson(path.join(launcherDir, 'package.json'), { ...launcherManifest, version, optionalDependencies })
copyLicenses(launcherDir)
const launcherTarball = pack(launcherDir)

fs.rmSync(workDir, { recursive: true, force: true })
console.log([...platformTarballs, launcherTarball].join('\n'))

function archivesByTarget() {
  const byTarget = new Map()
  for (const file of fs.readdirSync(archivesDir)) {
    const match = file.match(/^stratum-lint-(.+)\.(tar\.gz|zip)$/)
    if (match) {
      byTarget.set(match[1], path.join(archivesDir, file))
    }
  }
  if (byTarget.size === 0) {
    die(`${archivesDir} holds no stratum-lint-<target>.tar.gz or .zip`)
  }
  return byTarget
}

function copyExecutables(file, fileNames, packageDir) {
  const extractDir = path.join(workDir, '.extract', path.basename(file))
  fs.mkdirSync(extractDir, { recursive: true })
  extract(file, extractDir)
  for (const fileName of fileNames) {
    const found = findFile(extractDir, fileName) ?? die(`${file} holds no ${fileName}, which the launcher runs`)
    const target = path.join(packageDir, 'bin', fileName)
    fs.copyFileSync(found, target)
    fs.chmodSync(target, 0o755)
  }
}

// GNU tar (Git Bash, Linux) cannot read .zip; Windows' own bsdtar reads both formats.
function extract(file, dir) {
  const windowsTar = process.env.SystemRoot && path.join(process.env.SystemRoot, 'System32', 'tar.exe')
  const [command, args] = file.endsWith('.zip')
    ? process.platform === 'win32'
      ? [windowsTar, ['-xf', file, '-C', dir]]
      : ['unzip', ['-q', '-o', file, '-d', dir]]
    : [process.platform === 'win32' ? windowsTar : 'tar', ['-xzf', file, '-C', dir]]
  run(command, args)
}

function findFile(dir, fileName) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const entryPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      const found = findFile(entryPath, fileName)
      if (found) {
        return found
      }
    } else if (entry.name === fileName) {
      return entryPath
    }
  }
  return undefined
}

function copyLicenses(packageDir) {
  for (const entry of fs.readdirSync(repoRoot)) {
    if (/^LICENSE/i.test(entry)) {
      fs.copyFileSync(path.join(repoRoot, entry), path.join(packageDir, entry))
    }
  }
}

function pack(packageDir) {
  // npm is npm.cmd on Windows, which Node runs only through a shell (CVE-2024-27980).
  const stdout = run('npm', ['pack', '--json', '--silent'], { cwd: packageDir, shell: process.platform === 'win32' })
  const [{ filename }] = JSON.parse(stdout)
  const tarball = path.join(outDir, filename)
  fs.renameSync(path.join(packageDir, filename), tarball)
  return tarball
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], ...options })
  if (result.error || result.status !== 0) {
    die(`${command} ${args.join(' ')} failed: ${result.error?.message ?? `exit ${result.status}`}`)
  }
  return result.stdout
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'))
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`)
}

function die(message) {
  console.error(`npm-pack: ${message}`)
  process.exit(1)
}
