#!/usr/bin/env node
'use strict'

const fs = require('fs')
const path = require('path')
const https = require('https')
const { execSync } = require('child_process')

const pkg = require('./package.json')
const VERSION = pkg.version

const TARGETS = {
  'linux-x64': 'stratum-lint-x86_64-unknown-linux-gnu.tar.gz',
  'linux-arm64': 'stratum-lint-aarch64-unknown-linux-gnu.tar.gz',
  'darwin-x64': 'stratum-lint-x86_64-apple-darwin.tar.gz',
  'darwin-arm64': 'stratum-lint-aarch64-apple-darwin.tar.gz',
  'win32-x64': 'stratum-lint-x86_64-pc-windows-msvc.zip',
}

const platform = `${process.platform}-${process.arch}`
const asset = TARGETS[platform]
if (!asset) {
  console.error(`[stratum-lint] unsupported platform ${platform}`)
  process.exit(1)
}

const url = `https://github.com/PetukhovArt/stratum/releases/download/v${VERSION}/${asset}`
const tmp = path.join(__dirname, asset)
const binaryDir = path.join(__dirname, 'binary')
fs.mkdirSync(binaryDir, { recursive: true })

const download = (target, dest) =>
  new Promise((resolve, reject) => {
    const req = https.get(target, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        download(res.headers.location, dest).then(resolve, reject)
        return
      }
      if (res.statusCode !== 200) {
        reject(new Error(`download failed: ${res.statusCode} for ${target}`))
        return
      }
      const file = fs.createWriteStream(dest)
      res.pipe(file)
      file.on('finish', () => file.close(resolve))
      file.on('error', reject)
    })
    req.on('error', reject)
  })

;(async () => {
  try {
    await download(url, tmp)
    if (asset.endsWith('.zip')) {
      execSync(`tar -xf "${tmp}" -C "${binaryDir}"`, { stdio: 'inherit' })
    } else {
      execSync(`tar -xzf "${tmp}" -C "${binaryDir}"`, { stdio: 'inherit' })
    }
    fs.unlinkSync(tmp)
    console.log(`[stratum-lint] installed binary v${VERSION} for ${platform}`)
  } catch (e) {
    console.error(`[stratum-lint] install failed: ${e.message}`)
    process.exit(1)
  }
})()
