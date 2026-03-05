# `@nexus-render/core`

High-performance **headless 3D rendering** library for Node.js, written in Rust using [`wgpu`](https://wgpu.rs/) and exposed to TypeScript via [`napi-rs`](https://napi.rs/).

## Features

- **100% headless** – runs in pure Node.js, Docker, or cloud servers with no display required
- **GPU accelerated** – leverages native GPU via `wgpu` (Vulkan/Metal/DX12) with software fallback
- **GLTF/GLB loading** – load 3D assets from the file system
- **Built-in primitives** – cube, sphere, cylinder
- **Scene graph** – ID-based entity management with transform updates at 60 Hz+
- **Multiple output formats** – raw RGBA `Uint8Array`, JPEG buffer, and depth map (`Float32Array`)
- **Three.js compatible** – right-handed Y-up coordinate system, `(x, y, z, w)` quaternions

## Requirements

- Node.js >= 18
- Linux x86_64 (glibc) – additional platforms planned

## Installation

```bash
npm install @nexus-render/core
```

## Quick Start

```ts
import { RenderEngine } from '@nexus-render/core'

// Create a headless render engine (software fallback when no GPU is available)
const engine = new RenderEngine({ width: 1280, height: 720, enableGpu: false })

// Add a cube to the scene and place it above the origin
const cubeId = engine.addPrimitive('cube')
engine.setTransform(cubeId, [0, 1, 0], [0, 0, 0, 1])

// Add a directional light
engine.addDirectionalLight([0, -1, -0.5], 0.8)

// Point the camera at the cube
engine.setCamera([0, 2, 5], [0, 0, 0], 60)

// Render to a raw RGBA buffer
const pixels = engine.renderRaw('default')
// pixels is a Uint8Array of length width * height * 4

// Or render to JPEG
const jpeg = engine.renderFrameJpeg('default', 90)
// jpeg starts with the JPEG magic bytes FF D8 FF

// Or load a GLTF/GLB model
const modelId = engine.loadModel('/path/to/model.glb')
```

## API

### `new RenderEngine(options)`

| Option | Type | Description |
|--------|------|-------------|
| `width` | `number` | Render target width in pixels |
| `height` | `number` | Render target height in pixels |
| `enableGpu` | `boolean` | Request a physical GPU; falls back to software if unavailable |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `addPrimitive(type)` | `string` | Add a built-in primitive (`"cube"` or `"sphere"`) and return its ID |
| `setTransform(id, position, rotation)` | `void` | Set world-space position `[x,y,z]` and quaternion rotation `[x,y,z,w]` |
| `setCamera(position, target, fovDegrees)` | `void` | Configure the virtual camera |
| `addDirectionalLight(direction, intensity)` | `void` | Add a directional light |
| `loadModel(filePath)` | `string` | Load a GLTF/GLB file and return its scene ID |
| `renderRaw(cameraId)` | `Uint8Array` | Render to raw RGBA pixel buffer (`width × height × 4` bytes) |
| `renderFrameJpeg(cameraId, quality)` | `Uint8Array` | Render and compress to JPEG (quality 1–100) |
| `renderDepth(cameraId)` | `Float32Array` | Render depth map (`width × height` values in [0, 1]) |

### `sum(a, b)`

Utility function exported for build-pipeline verification.

## License

[MIT](./LICENSE)
