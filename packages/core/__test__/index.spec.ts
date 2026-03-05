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
