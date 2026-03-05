'use strict'

// napi-rs loader: tries the locally built binary first, then falls back to the
// platform-specific npm package (used in production installs).
const { existsSync, readFileSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

let nativeBinding = null
let localFileExisted = false
let loadError = null

function isMusl() {
  // For Node.js it is possible to detect musl via the ldd output.
  if (!process.report || typeof process.report.getReport !== 'function') {
    try {
      const lddPath = require('child_process').execSync('which ldd').toString().trim()
      return readFileSync(lddPath, 'utf8').includes('musl')
    } catch {
      return false
    }
  }
  const report = process.report.getReport()
  const glibcVersionRuntime =
    report && report.header && report.header.glibcVersionRuntime
  return !glibcVersionRuntime
}

switch (platform) {
  case 'android':
    switch (arch) {
      case 'arm64':
        localFileExisted = existsSync(join(__dirname, 'nexus-render-core.android-arm64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./nexus-render-core.android-arm64.node')
          } else {
            nativeBinding = require('@nexus-render/core-android-arm64')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on Android: ${arch}`)
    }
    break
  case 'win32':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(join(__dirname, 'nexus-render-core.win32-x64-msvc.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./nexus-render-core.win32-x64-msvc.node')
          } else {
            nativeBinding = require('@nexus-render/core-win32-x64-msvc')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on Windows: ${arch}`)
    }
    break
  case 'darwin':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(join(__dirname, 'nexus-render-core.darwin-x64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./nexus-render-core.darwin-x64.node')
          } else {
            nativeBinding = require('@nexus-render/core-darwin-x64')
          }
        } catch (e) {
          loadError = e
        }
        break
      case 'arm64':
        localFileExisted = existsSync(join(__dirname, 'nexus-render-core.darwin-arm64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./nexus-render-core.darwin-arm64.node')
          } else {
            nativeBinding = require('@nexus-render/core-darwin-arm64')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on macOS: ${arch}`)
    }
    break
  case 'freebsd':
    if (arch !== 'x64') {
      throw new Error(`Unsupported architecture on FreeBSD: ${arch}`)
    }
    localFileExisted = existsSync(join(__dirname, 'nexus-render-core.freebsd-x64.node'))
    try {
      if (localFileExisted) {
        nativeBinding = require('./nexus-render-core.freebsd-x64.node')
      } else {
        nativeBinding = require('@nexus-render/core-freebsd-x64')
      }
    } catch (e) {
      loadError = e
    }
    break
  case 'linux':
    switch (arch) {
      case 'x64':
        if (isMusl()) {
          localFileExisted = existsSync(join(__dirname, 'nexus-render-core.linux-x64-musl.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require('./nexus-render-core.linux-x64-musl.node')
            } else {
              nativeBinding = require('@nexus-render/core-linux-x64-musl')
            }
          } catch (e) {
            loadError = e
          }
        } else {
          localFileExisted = existsSync(join(__dirname, 'nexus-render-core.linux-x64-gnu.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require('./nexus-render-core.linux-x64-gnu.node')
            } else {
              nativeBinding = require('@nexus-render/core-linux-x64-gnu')
            }
          } catch (e) {
            loadError = e
          }
        }
        break
      case 'arm64':
        if (isMusl()) {
          localFileExisted = existsSync(join(__dirname, 'nexus-render-core.linux-arm64-musl.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require('./nexus-render-core.linux-arm64-musl.node')
            } else {
              nativeBinding = require('@nexus-render/core-linux-arm64-musl')
            }
          } catch (e) {
            loadError = e
          }
        } else {
          localFileExisted = existsSync(join(__dirname, 'nexus-render-core.linux-arm64-gnu.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require('./nexus-render-core.linux-arm64-gnu.node')
            } else {
              nativeBinding = require('@nexus-render/core-linux-arm64-gnu')
            }
          } catch (e) {
            loadError = e
          }
        }
        break
      default:
        throw new Error(`Unsupported architecture on Linux: ${arch}`)
    }
    break
  default:
    throw new Error(`Unsupported OS: ${platform}`)
}

if (!nativeBinding) {
  if (loadError) {
    throw loadError
  }
  throw new Error('Failed to load native binding')
}

const { sum, RenderEngine } = nativeBinding

module.exports.sum = sum
module.exports.RenderEngine = RenderEngine
