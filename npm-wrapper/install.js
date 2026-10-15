#!/usr/bin/env node
'use strict'

const fs = require('fs')
const path = require('path')
const https = require('https')
const { execSync } = require('child_process')

const pkg = require('./package.json')
const VERSION = pkg.version

const TARGETS = {
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'win32-x64': 'x86_64-pc-windows-msvc',
}
const BINARIES = ['stratum-lint', 'stratum-lsp']

const platform = `${process.platform}-${process.arch}`
const target = TARGETS[platform]
if (!target) {
  console.error(`[stratum-lint] unsupported platform ${platform}`)
  process.exit(1)
}

const exe = process.platform === 'win32' ? '.exe' : ''
const archiveExt = process.platform === 'win32' ? '.zip' : '.tar.gz'
const binaryDir = path.join(__dirname, 'binary')
fs.mkdirSync(binaryDir, { recursive: true })

const download = (url, dest) =>
  new Promise((resolve, reject) => {
    const req = https.get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        download(res.headers.location, dest).then(resolve, reject)
        return
      }
      if (res.statusCode !== 200) {
        res.resume()
        reject(new Error(`download failed: ${res.statusCode} for ${url}`))
        return
      }
      const file = fs.createWriteStream(dest)
      res.pipe(file)
      file.on('finish', () => file.close(resolve))
      file.on('error', reject)
    })
    req.on('error', reject)
  })

const findFile = (dir, name) => {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isFile() && entry.name === name) return full
    if (entry.isDirectory()) {
      const found = findFile(full, name)
      if (found) return found
    }
  }
  return null
}

const installArchive = async (app) => {
  const asset = `${app}-${target}${archiveExt}`
  const tmp = path.join(__dirname, asset)
  await download(`https://github.com/PetukhovArt/stratum/releases/download/v${VERSION}/${asset}`, tmp)
  const flags = archiveExt === '.zip' ? '-xf' : '-xzf'
  execSync(`tar ${flags} "${tmp}" -C "${binaryDir}"`, { stdio: 'inherit' })
  fs.unlinkSync(tmp)
}

const placeBinary = (name) => {
  const file = `${name}${exe}`
  const found = findFile(binaryDir, file)
  if (!found) return false
  const dest = path.join(binaryDir, file)
  if (found !== dest) fs.renameSync(found, dest)
  return true
}

;(async () => {
  try {
    await installArchive('stratum-lint')
    for (const name of BINARIES) {
      if (!placeBinary(name)) {
        await installArchive(name)
        if (!placeBinary(name)) throw new Error(`${name}${exe} not found in release archives`)
      }
    }
    console.log(`[stratum-lint] installed ${BINARIES.join(', ')} v${VERSION} for ${platform}`)
  } catch (e) {
    console.error(`[stratum-lint] install failed: ${e.message}`)
    process.exit(1)
  }
})()
