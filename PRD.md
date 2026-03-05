## 1. Project Context & General Description

**Context:** We are building `@nexus-render/core`, a high-performance, 100% headless 3D rendering library for Node.js. It is written in Rust using `wgpu` and exposed to TypeScript via `napi-rs`.

**Goal:** The library allows Node.js backend servers to load 3D assets (GLTF/GLB), construct a scene, manipulate object transforms in real-time, and render the output (from virtual cameras) directly to raw pixel buffers or compressed JPEGs. It operates entirely without a browser, DOM, or windowing system.

**Interoperability:** The library must be easily usable alongside ecosystem standards like Three.js. This means adopting a **Right-Handed, Y-Up coordinate system**, utilizing standard Quaternions $(x, y, z, w)$ for rotations, and matching Three.js scene graph logic where applicable.

---

## 2. Product Requirements Document (PRD)

### Core Objectives

* **100% Headless:** Must run in pure Node.js background processes, Docker, or cloud servers without windowing systems (X11/Wayland/Quartz) unless required by standard Vulkan/Metal/DX12 drivers.
* **Agnostic & Unopinionated:** Operates purely on meshes, transforms, and cameras. It does not know about React, ROS, or physics engines.
* **High Performance:** Leverages native GPU acceleration via `wgpu`, with a fallback to software rendering (e.g., `lavapipe`) if no physical GPU is present. Zero-copy or minimal-copy memory transfers between Rust and Node.js using `napi-rs`.

### Technical Stack

* **Core Graphics:** Rust, `wgpu`, `glam` (for fast 3D math), `gltf` (for parsing).
* **Node.js Bridge:** `napi-rs` (specifically `@napi-rs/cli` for scaffolding).
* **Image Processing:** Rust `image` crate (for fast JPEG compression/depth mapping).
* **API:** TypeScript (Strict mode), Jest (for TDD).

### Feature Scope

* **Asset Loading:** GLTF/GLB file support; built-in primitives (Cube, Sphere, Cylinder).
* **Scene Graph:** Flat or hierarchical ID-based entity management. Transform updates (Translation, Rotation via Quaternions, Scale) capable of running at 60Hz.
* **Lighting:** Ambient and Directional lights.
* **Camera & Output:** Multi-camera support. Configurable intrinsics (FOV, Resolution, Near/Far clip). Output to `Uint8Array` (Raw RGBA), JPEG buffers, and Depth maps.

---

## 3. TDD Implementation Steps

*Note for AI: Follow these steps sequentially. For every step, write the TypeScript Jest test **first** to define the expected behavior, then implement the necessary Rust/N-API code to make the test pass.*

### Step 1: Project Scaffolding & CI

1. **Initialize the monorepo:** Use the NAPI-RS CLI to scaffold a new project named `@nexus-render/core`.
2. **Configure TypeScript and Jest:** Ensure the TS configuration is set to strict. Set up Jest for testing Node.js APIs.
3. **Rust Setup:** Add `wgpu`, `napi`, `napi-derive`, and `tokio` to the `Cargo.toml`.
4. **TDD Action:** Write a dummy Jest test checking if a basic Rust function (e.g., `sum(a, b)`) can be called from TypeScript. Implement the Rust function and ensure the build pipeline works.

### Step 2: Headless WGPU Initialization

1. **Define the TS API:** Create the `RenderEngine` class with a constructor accepting `{ width, height, enableGPU }`.
2. **TDD Action:** Write a test that instantiates `new RenderEngine(...)` and verifies it does not throw an error.
3. **Rust Implementation:** In Rust, implement the `RenderEngine` struct. Write the initialization logic to request a `wgpu::Instance`, `wgpu::Adapter`, `wgpu::Device`, and `wgpu::Queue`. Configure the instance to be explicitly headless. Store this state inside the N-API class wrapper.

### Step 3: Scene Graph & Transform State Management

1. **Define the TS API:** Add methods `engine.addPrimitive('cube')`, `engine.setTransform(id, position, rotation)`. Keep rotation as a Quaternion to match Three.js.
2. **TDD Action:** Write tests that add an object, return a unique numerical/string ID, and successfully update its transform without throwing.
3. **Rust Implementation:** Create an internal ECS (Entity Component System) or a simple `HashMap` to store scene objects, their types, and their `glam::Mat4` transform matrices.

### Step 4: The Offscreen Render Pipeline & Buffer Extraction

1. **Define the TS API:** Add the `engine.renderRaw(cameraId)` method returning a `Uint8Array`.
2. **TDD Action:** Write a test that initializes the engine, calls `renderRaw`, and verifies the output is a `Uint8Array` of length `width * height * 4`.
3. **Rust Implementation:** * Create a `wgpu::Texture` with `TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC`.
* Create a destination `wgpu::Buffer` in GPU memory to copy the texture to.
* Set up a dummy RenderPass that clears the screen to a specific color (e.g., bright red).
* Submit the command, map the buffer asynchronously, and convert the bytes to an N-API `Buffer` / `Uint8Array` to pass back to Node.



### Step 5: Rendering Primitives & Shaders

1. **TDD Action:** Write a test that adds a cube to the center of the scene, places a camera, renders the frame, and checks specific pixel values (to verify the cube is actually visible, not just the clear color).
2. **Rust Implementation:** * Write a basic WGSL shader (`shader.wgsl`) with vertex and fragment stages.
* Implement vertex buffers and index buffers for the basic primitives.
* Implement the Camera Uniform Buffer Object (UBO) to pass the View-Projection matrix to the shader.



### Step 6: Three.js Coordinate Alignment & Lighting

1. **Define the TS API:** Add `engine.addDirectionalLight(direction, intensity)`.
2. **TDD Action:** Write tests verifying that moving an object on the +Y axis moves it *up* (Three.js standard), and +Z moves it *towards* the camera.
3. **Rust Implementation:** * Update the WGSL shaders to support basic Lambertian or Blinn-Phong lighting.
* Ensure the projection matrix in Rust uses a Right-Handed system (e.g., using `glam::Mat4::perspective_rh_gl`).



### Step 7: GLTF/GLB Asset Loading

1. **Define the TS API:** Add `engine.loadModel(filePath)`.
2. **TDD Action:** Write a test that loads a tiny, valid `.glb` file from the local file system and verifies an ID is returned.
3. **Rust Implementation:** Use the `gltf` crate to parse the file. Extract the vertex data, normals, and indices, upload them to `wgpu` buffers, and store the mesh data in the scene graph.

### Step 8: Image Compression & Depth Output

1. **Define the TS API:** Add `engine.renderFrameJpeg(cameraId, quality)` and `engine.renderDepth(cameraId)`.
2. **TDD Action:** Write a test that renders a JPEG, checks the magic bytes of the output buffer to ensure it is a valid JPEG header, and checks that the depth map returns a valid single-channel buffer.
3. **Rust Implementation:** * Use the `image` crate. Take the raw RGBA buffer, encode it as a JPEG in memory, and return that byte array to Node.js.
* Extract the `wgpu::TextureFormat::Depth32Float` attachment to a buffer and pass it back.
