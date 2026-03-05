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
