# nexus-render

**Server-side 3D rendering for Node.js.** Load 3D assets, build scenes, and render frames — entirely on the backend, with no browser or display required.

Built in Rust with [wgpu](https://wgpu.rs/), exposed to TypeScript via [napi-rs](https://napi.rs/). GPU-accelerated with automatic software fallback.

---

## Why nexus-render?

Most 3D rendering happens in the browser. nexus-render brings that capability to the server, enabling use cases that are impossible with WebGL or Three.js alone:

- **Server-side thumbnail generation** — render previews of 3D models (GLB/GLTF) at upload time, no client needed
- **Automated visual testing** — render scenes headlessly in CI and diff pixel output against golden images
- **Robotics & simulation** — generate synthetic depth maps or RGB frames for training data pipelines
- **Product configurators** — pre-render product variants server-side and cache the results
- **Digital twins** — render real-time state of physical systems into image streams for dashboards

---

## Packages

| Package | Description |
|---|---|
| [`@nexus-render/core`](./packages/core) | Core rendering engine — headless wgpu renderer for Node.js |

---

## Installation

```bash
npm install @nexus-render/core
```

> Requires Node.js >= 18. Currently supports Linux x86_64 (glibc). macOS and Windows support planned.

---

## Examples

### Render a primitive to JPEG

```ts
import { RenderEngine } from '@nexus-render/core'
import { writeFileSync } from 'fs'

const engine = new RenderEngine({ width: 1280, height: 720, enableGpu: false })

const cubeId = engine.addPrimitive('cube')
engine.setTransform(cubeId, [0, 0, 0], [0, 0, 0, 1])
engine.addDirectionalLight([0, -1, -0.5], 0.8)
engine.setCamera([0, 2, 5], [0, 0, 0], 60)

const jpeg = engine.renderFrameJpeg('default', 90)
writeFileSync('output.jpg', jpeg)
```

### Load a GLB model and render a thumbnail

```ts
import { RenderEngine } from '@nexus-render/core'

const engine = new RenderEngine({ width: 512, height: 512, enableGpu: false })

const modelId = engine.loadModel('/uploads/chair.glb')
engine.setTransform(modelId, [0, 0, 0], [0, 0, 0, 1])
engine.addDirectionalLight([0.5, -1, -0.5], 0.9)
engine.setCamera([2, 2, 4], [0, 0, 0], 45)

const jpeg = engine.renderFrameJpeg('default', 85)
// → serve jpeg as HTTP response or store in object storage
```

### Generate a depth map for a scene

```ts
import { RenderEngine } from '@nexus-render/core'

const engine = new RenderEngine({ width: 640, height: 480, enableGpu: false })

const modelId = engine.loadModel('/scene/environment.glb')
engine.setCamera([0, 1.6, 0], [0, 1.6, -1], 90) // first-person camera

const depth = engine.renderDepth('default')
// → Float32Array of length 640 * 480
// values are in [0.0, 1.0] — 0 = near plane, 1 = far plane
```

### Use with Three.js transforms

nexus-render uses the same coordinate system and quaternion convention as Three.js, so you can pass transforms directly:

```ts
import * as THREE from 'three'
import { RenderEngine } from '@nexus-render/core'

const engine = new RenderEngine({ width: 1920, height: 1080, enableGpu: true })

// Use Three.js to compute transforms, pass them straight to nexus-render
const obj = new THREE.Object3D()
obj.position.set(1, 0.5, -2)
obj.rotation.set(0, Math.PI / 4, 0)

const q = new THREE.Quaternion().setFromEuler(obj.rotation)
const modelId = engine.loadModel('/assets/model.glb')

engine.setTransform(
  modelId,
  [obj.position.x, obj.position.y, obj.position.z],
  [q.x, q.y, q.z, q.w],
)
```

---

## Development

This is a Cargo + npm monorepo.

```bash
# Install JS dependencies
npm install

# Build the native module (requires Rust toolchain)
npm run build

# Run tests
npm run test

# Dry-run publish (preview what gets uploaded)
npm run publish:dry

# Build + test + publish
npm run release
```

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (`cargo`, `rustc`)
- Node.js >= 18
- `@napi-rs/cli` (installed automatically via `npm install`)

---

## Architecture

```
nexus-render/
├── packages/
│   └── core/           # @nexus-render/core — napi-rs Rust + TypeScript package
│       ├── src/
│       │   ├── lib.rs       # napi-rs bindings & RenderEngine struct
│       │   ├── world.rs     # scene graph, ECS, transforms
│       │   └── shader.wgsl  # WGSL vertex + fragment shaders
│       ├── index.js         # JS wrapper (safe copy of depth buffer, render-lock guard)
│       └── index.d.ts       # TypeScript declarations
└── Cargo.toml           # Rust workspace root
```

---

## License

[MIT](./packages/core/LICENSE)
