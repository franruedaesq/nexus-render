// ── Uniform blocks ────────────────────────────────────────────────────────────

/// Camera uniform: the combined View-Projection matrix.
struct CameraUniforms {
    view_proj : mat4x4<f32>,
}

/// Per-object uniform: model matrix and its normal matrix
/// (transpose of the inverse of the model matrix).
struct ModelUniforms {
    model      : mat4x4<f32>,
    normal_mat : mat4x4<f32>,
}

/// Directional-light uniform.
/// `direction` points FROM the light source TOWARD the scene (Three.js convention).
/// `intensity` is a scalar in [0, 1].
struct LightUniforms {
    direction : vec3<f32>,
    intensity : f32,
}

@group(0) @binding(0) var<uniform> camera  : CameraUniforms;
@group(0) @binding(1) var<uniform> model_u : ModelUniforms;
@group(0) @binding(2) var<uniform> light   : LightUniforms;

// ── Vertex stage ──────────────────────────────────────────────────────────────

struct VertexInput {
    @location(0) position : vec3<f32>,
    @location(1) normal   : vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos    : vec4<f32>,
    @location(0)       world_normal : vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out : VertexOutput;
    let world_pos     = model_u.model * vec4<f32>(in.position, 1.0);
    out.clip_pos      = camera.view_proj * world_pos;
    // Transform the normal into world space using the pre-computed normal matrix.
    out.world_normal  = normalize((model_u.normal_mat * vec4<f32>(in.normal, 0.0)).xyz);
    return out;
}

// ── Fragment stage ────────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(in.world_normal);
    // `light.direction` points toward the surface; negate to get the
    // surface-to-light vector required by the Lambertian diffuse formula.
    let L       = normalize(-light.direction);
    let diffuse = max(dot(N, L), 0.0) * light.intensity;
    let ambient = 0.15;
    let bright  = clamp(ambient + diffuse, 0.0, 1.0);
    // Render as a white/grey surface shaded by Lambertian lighting.
    return vec4<f32>(bright, bright, bright, 1.0);
}
