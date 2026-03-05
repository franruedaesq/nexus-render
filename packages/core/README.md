# `@nexus-render/core`

High-performance **headless 3D rendering** for Node.js — written in Rust with [`wgpu`](https://wgpu.rs/), exposed to TypeScript via [`napi-rs`](https://napi.rs/).

Render 3D scenes entirely on the server: no browser, no display, no DOM.

## Features

- **100% headless** — runs in Node.js, Docker, or cloud servers with no display required
- **GPU accelerated** — uses native GPU via wgpu (Vulkan / Metal / DX12) with automatic software fallback
- **GLTF/GLB loading** — load real 3D assets from disk
- **Built-in primitives** — cube and sphere out of the box
- **Multiple output formats** — raw RGBA `Uint8Array`, JPEG buffer, depth map `Float32Array`
- **Three.js compatible** — right-handed Y-up coordinate system, `[x, y, z, w]` quaternions

## Requirements

- Node.js >= 18
- Linux x86_64 (glibc) — additional platforms planned

## Installation

```bash
npm install @nexus-render/core
```

## Quick Start

```ts
import { RenderEngine } from '@nexus-render/core'

const engine = new RenderEngine({ width: 1280, height: 720, enableGpu: false })

// Build a scene
const cubeId = engine.addPrimitive('cube')
engine.setTransform(cubeId, [0, 0, 0], [0, 0, 0, 1])
engine.addDirectionalLight([0, -1, -0.5], 0.8)
engine.setCamera([0, 2, 5], [0, 0, 0], 60)

// Render to raw RGBA
const pixels = engine.renderRaw('default')
// → Uint8Array of length width * height * 4

// Render to JPEG
const jpeg = engine.renderFrameJpeg('default', 90)
// → Uint8Array starting with JPEG magic bytes FF D8 FF

// Render depth map
const depth = engine.renderDepth('default')
// → Float32Array of length width * height, values in [0.0, 1.0]

// Load a GLTF/GLB model
const modelId = engine.loadModel('/path/to/model.glb')
engine.setTransform(modelId, [1, 0, 0], [0, 0, 0, 1])
```

## API

### `new RenderEngine(options)`

| Option | Type | Description |
|---|---|---|
| `width` | `number` | Render target width in pixels |
| `height` | `number` | Render target height in pixels |
| `enableGpu` | `boolean` | Request a physical GPU; falls back to software if unavailable |

### Methods

| Method | Returns | Description |
|---|---|---|
| `addPrimitive(type)` | `number` | Add a built-in primitive (`"cube"` or `"sphere"`), returns a unique entity ID |
| `setTransform(id, position, rotation)` | `void` | Set world-space position `[x,y,z]` and quaternion `[x,y,z,w]` |
| `setCamera(position, target, fovDegrees)` | `void` | Configure the virtual camera |
| `addDirectionalLight(direction, intensity)` | `void` | Add a directional light |
| `loadModel(filePath)` | `number` | Load a GLTF/GLB file, returns a unique entity ID |
| `renderRaw(cameraId)` | `Uint8Array` | Raw RGBA pixel buffer — `width × height × 4` bytes |
| `renderFrameJpeg(cameraId, quality)` | `Uint8Array` | JPEG-encoded frame — quality range 1–100 |
| `renderDepth(cameraId)` | `Float32Array` | Depth map — `width × height` values in `[0.0, 1.0]` |

### Coordinate system

Matches Three.js conventions:
- Right-handed, Y-up
- `+X` = right, `+Y` = up, `+Z` = toward the viewer
- Quaternions in `[x, y, z, w]` order

## License

[MIT](./LICENSE)
