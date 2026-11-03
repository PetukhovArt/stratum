#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const manifest = JSON.parse(fs.readFileSync(path.join(repoRoot, 'npm', 'stratum-lint', 'package.json'), 'utf8'))
const homepage = manifest.homepage
const description = 'Stratum architectural linter'

const args = process.argv.slice(2)
const tag = args[0]
if (!tag || !/^v\d+\.\d+\.\d+$/.test(tag)) die('usage: node scripts/bump-packages.mjs vX.Y.Z [--tap DIR] [--bucket DIR]')
const tapDir = option('--tap')
const bucketDir = option('--bucket')
if (!tapDir && !bucketDir) die('pass --tap DIR, --bucket DIR or both')
const version = tag.slice(1)
const asset = (target, ext) => `${homepage}/releases/download/${tag}/stratum-lint-${target}.${ext}`

if (tapDir) {
  const targets = {
    macArm: asset('aarch64-apple-darwin', 'tar.gz'),
    macIntel: asset('x86_64-apple-darwin', 'tar.gz'),
    linuxArm: asset('aarch64-unknown-linux-gnu', 'tar.gz'),
    linuxIntel: asset('x86_64-unknown-linux-gnu', 'tar.gz'),
  }
  const sha = Object.fromEntries(await Promise.all(Object.entries(targets).map(async ([k, url]) => [k, await sha256(url)])))
  const block = (url, hash) => `      url "${url}"\n      sha256 "${hash}"`
  const formula = `class StratumLint < Formula
  desc "${description}"
  homepage "${homepage}"
  version "${version}"
  license "MIT"

  on_macos do
    on_arm do
${block(targets.macArm, sha.macArm)}
    end
    on_intel do
${block(targets.macIntel, sha.macIntel)}
    end
  end

  on_linux do
    on_arm do
${block(targets.linuxArm, sha.linuxArm)}
    end
    on_intel do
${block(targets.linuxIntel, sha.linuxIntel)}
    end
  end

  def install
    bin.install "stratum-lint"
    bin.install "stratum-lsp" if File.exist?("stratum-lsp")
  end

  test do
    assert_match "stratum-lint", shell_output("#{bin}/stratum-lint --help")
  end
end
`
  fs.mkdirSync(path.join(tapDir, 'Formula'), { recursive: true })
  fs.writeFileSync(path.join(tapDir, 'Formula', 'stratum-lint.rb'), formula)
  commit(tapDir, 'Formula/stratum-lint.rb')
}

if (bucketDir) {
  const x64 = asset('x86_64-pc-windows-msvc', 'zip')
  const arm64 = asset('aarch64-pc-windows-msvc', 'zip')
  const auto = (target) => `${homepage}/releases/download/v$version/stratum-lint-${target}.zip`
  const scoop = {
    version,
    description,
    homepage,
    license: 'MIT',
    architecture: {
      '64bit': { url: x64, hash: await sha256(x64) },
      arm64: { url: arm64, hash: await sha256(arm64) },
    },
    bin: ['stratum-lint.exe', 'stratum-lsp.exe'],
    checkver: 'github',
    autoupdate: {
      architecture: {
        '64bit': { url: auto('x86_64-pc-windows-msvc') },
        arm64: { url: auto('aarch64-pc-windows-msvc') },
      },
    },
  }
  fs.writeFileSync(path.join(bucketDir, 'stratum-lint.json'), `${JSON.stringify(scoop, null, 2)}\n`)
  commit(bucketDir, 'stratum-lint.json')
}

function option(name) {
  const i = args.indexOf(name)
  return i === -1 ? undefined : (args[i + 1] ?? die(`${name} needs a directory`))
}

async function sha256(url) {
  console.log(`fetching ${url}`)
  const response = await fetch(url)
  if (!response.ok) die(`${url}: HTTP ${response.status}`)
  return createHash('sha256').update(Buffer.from(await response.arrayBuffer())).digest('hex')
}

function commit(dir, file) {
  for (const gitArgs of [['add', file], ['commit', '-m', `stratum-lint ${version}`]]) {
    const result = spawnSync('git', gitArgs, { cwd: dir, stdio: 'inherit' })
    if (result.status !== 0) die(`git ${gitArgs.join(' ')} failed in ${dir}`)
  }
  console.log(`ready to push: cd ${dir} && git push`)
}

function die(message) {
  console.error(`bump-packages: ${message}`)
  process.exit(1)
}
