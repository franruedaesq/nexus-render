'use strict'

/**
 * TypeScript wrapper around the native `@nexus-render/core` NAPI module.
 *
 * Purpose
 * -------
 * The native module exposes a raw `Float32Array` from `renderDepth`. Because
 * this array references memory inside the native module's heap, it is **not
 * safe** to hold on to the reference across subsequent native calls that may
 * trigger a heap reallocation (invalidating the pointer). This wrapper class:
 *
 * 1. Copies depth data into a plain JavaScript `Float32Array` before returning
 *    it, so callers can safely store the result indefinitely.
 * 2. Tracks whether a render is currently in progress and throws a descriptive
 *    error if another native render method is called concurrently.
 * 3. Re-exports all other methods unchanged, providing a single clean entry
 *    point for TypeScript consumers with full type information.
 */

const { existsSync, readFileSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

let nativeBinding = null
let localFileExisted = false
let loadError = null

function isMusl() {
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

const { sum: _sum, RenderEngine: _NativeRenderEngine } = nativeBinding

/**
 * Safe wrapper around the native `RenderEngine`.
 *
 * Key safety guarantees compared to using the native class directly:
 *
 * - **`renderDepth` returns a safe copy**: The raw `Float32Array` exposed by
 *   the native layer points directly into native memory. Holding a reference
 *   to that array across subsequent native calls risks accessing freed or
 *   reallocated memory. This wrapper copies the data into a fresh JavaScript
 *   `Float32Array` before returning it, making the result safe to store.
 *
 * - **Render-lock guard**: A boolean flag (`_rendering`) is set while any
 *   render call is executing. Any attempt to start a second render while one
 *   is already in progress throws a descriptive error rather than silently
 *   corrupting state.
 */
class RenderEngine {
  constructor(options) {
    this._engine = new _NativeRenderEngine(options)
    this._rendering = false
  }

  get width() {
    return this._engine.width
  }

  get height() {
    return this._engine.height
  }

  addPrimitive(primitiveType) {
    return this._engine.addPrimitive(primitiveType)
  }

  setTransform(id, position, rotation) {
    return this._engine.setTransform(id, position, rotation)
  }

  setCamera(position, target, fovDegrees) {
    return this._engine.setCamera(position, target, fovDegrees)
  }

  addDirectionalLight(direction, intensity) {
    return this._engine.addDirectionalLight(direction, intensity)
  }

  renderRaw(cameraId) {
    this._assertNotRendering()
    this._rendering = true
    try {
      return this._engine.renderRaw(cameraId)
    } finally {
      this._rendering = false
    }
  }

  loadModel(filePath) {
    return this._engine.loadModel(filePath)
  }

  renderFrameJpeg(cameraId, quality) {
    this._assertNotRendering()
    this._rendering = true
    try {
      return this._engine.renderFrameJpeg(cameraId, quality)
    } finally {
      this._rendering = false
    }
  }

  /**
   * Render the depth buffer and return a **safe copy** as a plain JavaScript
   * `Float32Array`.
   *
   * Unlike the raw native method, the returned array does not alias native
   * memory, so it is safe to store and access across subsequent native calls.
   */
  renderDepth(cameraId) {
    this._assertNotRendering()
    this._rendering = true
    try {
      const nativeView = this._engine.renderDepth(cameraId)
      // Copy into a plain JS Float32Array so the caller can safely hold
      // onto the result after this method returns.
      return new Float32Array(nativeView)
    } finally {
      this._rendering = false
    }
  }

  _assertNotRendering() {
    if (this._rendering) {
      throw new Error(
        'RenderEngine: cannot start a new render while another render is already in progress. ' +
        'Complete the previous render call before calling another render method.'
      )
    }
  }
}

module.exports.sum = _sum
module.exports.RenderEngine = RenderEngine
