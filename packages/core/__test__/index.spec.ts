import { sum, RenderEngine } from '../index'

// ─────────────────────────────────────────────────────────────────────────────
// Step 1 – Build Pipeline Smoke Test
// ─────────────────────────────────────────────────────────────────────────────

describe('sum (Step 1 – build pipeline)', () => {
  it('adds two positive integers', () => {
    expect(sum(1, 2)).toBe(3)
  })

  it('handles negative numbers', () => {
    expect(sum(-3, 5)).toBe(2)
  })

  it('handles both zero operands', () => {
    expect(sum(0, 0)).toBe(0)
  })
})

// ─────────────────────────────────────────────────────────────────────────────
// Step 2 – Headless RenderEngine Initialisation
// ─────────────────────────────────────────────────────────────────────────────

describe('RenderEngine (Step 2 – headless wgpu init)', () => {
  it('constructs without throwing using software rendering', () => {
    expect(() => {
      // force_fallback_adapter = true  →  always uses the software renderer
      new RenderEngine({ width: 640, height: 480, enableGpu: false })
    }).not.toThrow()
  })

  it('exposes correct width and height getters', () => {
    const engine = new RenderEngine({ width: 800, height: 600, enableGpu: false })
    expect(engine.width).toBe(800)
    expect(engine.height).toBe(600)
  })

  it('accepts different resolutions', () => {
    const engine = new RenderEngine({ width: 1920, height: 1080, enableGpu: false })
    expect(engine.width).toBe(1920)
    expect(engine.height).toBe(1080)
  })
})

// ─────────────────────────────────────────────────────────────────────────────
// Step 3 – Scene Graph & Transform State Management
// ─────────────────────────────────────────────────────────────────────────────

describe('RenderEngine (Step 3 – scene graph & transforms)', () => {
  it('addPrimitive returns a unique string ID for each call', () => {
    const engine = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    const id1 = engine.addPrimitive('cube')
    const id2 = engine.addPrimitive('cube')
    const id3 = engine.addPrimitive('sphere')

    expect(typeof id1).toBe('string')
    expect(typeof id2).toBe('string')
    expect(id1).not.toBe(id2)
    expect(id2).not.toBe(id3)
  })

  it('setTransform updates the transform without throwing', () => {
    const engine = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    const id = engine.addPrimitive('cube')
    // identity quaternion [0, 0, 0, 1] with a non-trivial translation
    expect(() => {
      engine.setTransform(id, [1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0])
    }).not.toThrow()
  })

  it('setTransform accepts a non-trivial rotation quaternion', () => {
    const engine = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    const id = engine.addPrimitive('sphere')
    // 90° rotation around Y axis: [0, sin(45°), 0, cos(45°)]
    const sin45 = Math.sin(Math.PI / 4)
    const cos45 = Math.cos(Math.PI / 4)
    expect(() => {
      engine.setTransform(id, [0.0, 0.0, 0.0], [0.0, sin45, 0.0, cos45])
    }).not.toThrow()
  })

  it('setTransform throws for an unknown primitive ID', () => {
    const engine = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    expect(() => {
      engine.setTransform('9999', [0, 0, 0], [0, 0, 0, 1])
    }).toThrow()
  })
})

// ─────────────────────────────────────────────────────────────────────────────
// Step 4 – Offscreen Render Pipeline & Buffer Extraction
// ─────────────────────────────────────────────────────────────────────────────

describe('RenderEngine (Step 4 – renderRaw)', () => {
  it('returns a Uint8Array of length width * height * 4', () => {
    const width = 64
    const height = 64
    const engine = new RenderEngine({ width, height, enableGpu: false })
    const buffer = engine.renderRaw('default')
    expect(buffer).toBeInstanceOf(Uint8Array)
    expect(buffer.length).toBe(width * height * 4)
  })

  it('pixel data is non-empty (clear color is bright red)', () => {
    const engine = new RenderEngine({ width: 4, height: 4, enableGpu: false })
    const buffer = engine.renderRaw('default')
    // The clear color is (1.0, 0.0, 0.0, 1.0) → RGBA8 (255, 0, 0, 255)
    // Verify at least one pixel has a non-zero red channel
    const hasNonZeroRed = Array.from(buffer).some((_, i) => i % 4 === 0 && buffer[i] > 0)
    expect(hasNonZeroRed).toBe(true)
  })

  it('works for different resolutions', () => {
    const w = 128
    const h = 96
    const engine = new RenderEngine({ width: w, height: h, enableGpu: false })
    const buffer = engine.renderRaw('default')
    expect(buffer.length).toBe(w * h * 4)
  })
})

// ─────────────────────────────────────────────────────────────────────────────
// Step 5 – Rendering Primitives & Shaders
// ─────────────────────────────────────────────────────────────────────────────

describe('RenderEngine (Step 5 – primitive rendering with shaders)', () => {
  it('renders a visible cube at the centre (centre pixel differs from clear colour)', () => {
    const width = 64
    const height = 64
    const engine = new RenderEngine({ width, height, enableGpu: false })

    // Add a cube at the world origin.
    const cubeId = engine.addPrimitive('cube')
    engine.setTransform(cubeId, [0, 0, 0], [0, 0, 0, 1])

    // Camera at (0, 0, 3) looking at the origin – the +Z face of the cube is
    // directly in view.
    engine.setCamera([0, 0, 3], [0, 0, 0], 60)

    // Directional light travelling in -Z, so it fully illuminates the +Z face.
    engine.addDirectionalLight([0, 0, -1], 0.8)

    const buffer = engine.renderRaw('default')

    // The centre pixel should show the shaded cube (white/grey), NOT the red
    // clear colour (255, 0, 0, 255).
    const cx = Math.floor(width / 2)
    const cy = Math.floor(height / 2)
    const idx = (cy * width + cx) * 4
    const r = buffer[idx]
    const g = buffer[idx + 1]
    const b = buffer[idx + 2]

    // A lit white cube renders as bright grey: all channels near 0.95 × 255 ≈ 242.
    // The clear colour is pure red (255, 0, 0), so at minimum green and blue
    // must be non-zero for the cube to be present.
    const isClearColour = r === 255 && g === 0 && b === 0
    expect(isClearColour).toBe(false)
    // Cube should be noticeably bright (ambient 0.15 + diffuse 0.8 = 0.95).
    expect(r).toBeGreaterThan(200)
    expect(g).toBeGreaterThan(200)
    expect(b).toBeGreaterThan(200)
  })

  it('setCamera does not throw for valid inputs', () => {
    const engine = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    expect(() => engine.setCamera([0, 0, 5], [0, 0, 0], 60)).not.toThrow()
  })

  it('addDirectionalLight does not throw for valid inputs', () => {
    const engine = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    expect(() => engine.addDirectionalLight([0, -1, 0], 0.8)).not.toThrow()
  })
})

// ─────────────────────────────────────────────────────────────────────────────
// Step 6 – Three.js Coordinate Alignment & Lighting
// ─────────────────────────────────────────────────────────────────────────────

describe('RenderEngine (Step 6 – coordinate system & lighting)', () => {
  /**
   * Helper that creates an engine, positions one cube, and renders.
   * Returns the raw RGBA buffer.
   */
  function renderCubeAt(
    position: [number, number, number],
    cameraPos: [number, number, number] = [0, 0, 3],
    lightDir: [number, number, number] = [0, 0, -1],
  ): Uint8Array {
    const engine = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    const id = engine.addPrimitive('cube')
    engine.setTransform(id, position, [0, 0, 0, 1])
    engine.setCamera(cameraPos, [0, 0, 0], 60)
    engine.addDirectionalLight(lightDir, 0.8)
    return engine.renderRaw('default')
  }

  function pixel(buf: Uint8Array, col: number, row: number): [number, number, number, number] {
    const i = (row * 64 + col) * 4
    return [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
  }

  const isClearColour = ([r, g, b]: [number, number, number, number]) =>
    r === 255 && g === 0 && b === 0

  // ── +Y axis moves the object upward (Three.js standard) ──────────────────

  it('+Y axis moves the cube upward in screen space', () => {
    // Move the cube well above the origin so it appears in the upper portion
    // of the 64×64 render.  With a 60° FOV camera at z=3, the half-height in
    // world space at z=0 is ≈ 1.73 units.  A cube centre at y=1.2 maps to
    // NDC_y ≈ 0.69, which is roughly row 10 from the top.
    const buf = renderCubeAt([0, 1.2, 0])

    // Row 10, centre column – inside the elevated cube.
    const topPx = pixel(buf, 32, 10)
    // Row 56, centre column – well below the cube, background region.
    const botPx = pixel(buf, 32, 56)

    // The bottom pixel must be the red clear colour (no cube there).
    expect(isClearColour(botPx)).toBe(true)
    // The top pixel must NOT be the red clear colour (cube is visible there).
    expect(isClearColour(topPx)).toBe(false)
  })

  // ── +Z axis moves the object toward the camera ────────────────────────────

  it('+Z axis moves the cube toward the camera (right-handed convention)', () => {
    // Camera at (0, 0, 3) looks in -Z.  A cube at (0, 0, 1.5) is between the
    // camera and the origin – it should remain visible and fill the centre.
    const buf = renderCubeAt([0, 0, 1.5])

    // Centre pixel must show the cube, not the clear colour.
    const centPx = pixel(buf, 32, 32)
    expect(isClearColour(centPx)).toBe(false)
    // A well-lit white cube will have all channels > 100.
    expect(centPx[0]).toBeGreaterThan(100)
    expect(centPx[1]).toBeGreaterThan(100)
    expect(centPx[2]).toBeGreaterThan(100)
  })

  // ── Directional light affects rendered brightness ─────────────────────────

  it('addDirectionalLight increases pixel brightness compared to no light', () => {
    // Render without any light (ambient only: brightness = 0.15).
    const engineNoLight = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    const id1 = engineNoLight.addPrimitive('cube')
    engineNoLight.setTransform(id1, [0, 0, 0], [0, 0, 0, 1])
    engineNoLight.setCamera([0, 0, 3], [0, 0, 0], 60)
    const bufNoLight = engineNoLight.renderRaw('default')

    // Render with a strong directional light (ambient 0.15 + diffuse 0.8).
    const engineLit = new RenderEngine({ width: 64, height: 64, enableGpu: false })
    const id2 = engineLit.addPrimitive('cube')
    engineLit.setTransform(id2, [0, 0, 0], [0, 0, 0, 1])
    engineLit.setCamera([0, 0, 3], [0, 0, 0], 60)
    engineLit.addDirectionalLight([0, 0, -1], 0.8)
    const bufLit = engineLit.renderRaw('default')

    // Sample the centre pixel on the cube's front face.
    const cx = 32
    const cy = 32
    const idxCenter = (cy * 64 + cx) * 4

    const brightnessNoLight = bufNoLight[idxCenter] // red channel of grey
    const brightnessLit = bufLit[idxCenter]

    // The lit render must be noticeably brighter than the ambient-only render.
    expect(brightnessLit).toBeGreaterThan(brightnessNoLight + 50)
  })
})
