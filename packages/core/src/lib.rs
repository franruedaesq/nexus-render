#![deny(clippy::all)]

use std::collections::HashMap;

use napi::bindgen_prelude::Uint8Array;
use napi_derive::napi;
use wgpu::util::DeviceExt;

/// Step 1: A simple sum function to verify the napi-rs build pipeline works.
#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
    a + b
}

/// Options for constructing a RenderEngine.
#[napi(object)]
pub struct RenderEngineOptions {
    pub width: u32,
    pub height: u32,
    pub enable_gpu: bool,
}

/// Internal representation of a scene object (not exposed via napi).
struct SceneObject {
    /// The primitive type (e.g. "cube", "sphere").
    primitive_type: String,
    /// The world-space transform matrix for this object.
    transform: glam::Mat4,
}

/// Camera state (internal). Defaults to an eye at (0, 0, 5) looking at the origin.
struct CameraState {
    position: glam::Vec3,
    target: glam::Vec3,
    fov_deg: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            position: glam::Vec3::new(0.0, 0.0, 5.0),
            target: glam::Vec3::ZERO,
            fov_deg: 60.0,
        }
    }
}

/// Directional light state (internal).
struct DirectionalLightState {
    /// Direction the light travels (from source toward the scene).
    direction: glam::Vec3,
    intensity: f32,
}

/// Step 2 / 3 / 4 / 5 / 6: Headless wgpu RenderEngine.
///
/// Initializes a wgpu Instance, Adapter, Device, and Queue in headless mode.
/// Falls back to software rendering (lavapipe) when no physical GPU is available.
/// Maintains an internal scene graph of primitives and their transforms.
#[napi]
pub struct RenderEngine {
    width: u32,
    height: u32,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// Scene objects keyed by their numeric ID.
    scene_objects: HashMap<u32, SceneObject>,
    /// Monotonically increasing counter used to assign unique IDs.
    next_id: u32,
    /// Step 5 / 6: Active camera.
    camera: CameraState,
    /// Step 6: Directional lights in the scene.
    directional_lights: Vec<DirectionalLightState>,
    /// Step 5: Compiled render pipeline (vertex + fragment shaders).
    render_pipeline: wgpu::RenderPipeline,
    /// Bind group layout for the render pipeline (camera / model / light uniforms).
    bind_group_layout: wgpu::BindGroupLayout,
}

#[napi]
impl RenderEngine {
    /// Create a new headless RenderEngine.
    ///
    /// # Arguments
    /// * `options.width`     – Render target width in pixels.
    /// * `options.height`    – Render target height in pixels.
    /// * `options.enableGpu` – When `true` prefer a hardware GPU; when `false`
    ///                         always request a software (CPU) adapter.
    #[napi(constructor)]
    pub fn new(options: RenderEngineOptions) -> napi::Result<Self> {
        let (device, queue) = pollster::block_on(init_wgpu(options.enable_gpu))
            .map_err(|e| napi::Error::from_reason(format!("wgpu init failed: {e}")))?;

        let (render_pipeline, bind_group_layout) = build_render_pipeline(&device);

        Ok(Self {
            width: options.width,
            height: options.height,
            device,
            queue,
            scene_objects: HashMap::new(),
            next_id: 0,
            camera: CameraState::default(),
            directional_lights: Vec::new(),
            render_pipeline,
            bind_group_layout,
        })
    }

    /// Returns the render target width.
    #[napi(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the render target height.
    #[napi(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Step 3: Add a primitive to the scene and return its unique string ID.
    ///
    /// # Arguments
    /// * `primitive_type` – One of `"cube"`, `"sphere"`, etc.
    ///
    /// # Returns
    /// A unique string ID that can be passed to `setTransform`.
    #[napi]
    pub fn add_primitive(&mut self, primitive_type: String) -> String {
        let id = self.next_id;
        self.next_id += 1;
        self.scene_objects.insert(
            id,
            SceneObject {
                primitive_type,
                transform: glam::Mat4::IDENTITY,
            },
        );
        id.to_string()
    }

    /// Step 3: Update the world-space transform of an existing scene object.
    ///
    /// # Arguments
    /// * `id`       – The string ID returned by `addPrimitive`.
    /// * `position` – `[x, y, z]` translation.
    /// * `rotation` – `[x, y, z, w]` unit quaternion (matches Three.js `Quaternion` component order).
    #[napi]
    pub fn set_transform(
        &mut self,
        id: String,
        position: Vec<f64>,
        rotation: Vec<f64>,
    ) -> napi::Result<()> {
        let id_num: u32 = id
            .parse()
            .map_err(|_| napi::Error::from_reason(format!("Invalid primitive ID: \"{id}\"")))?;

        if position.len() != 3 {
            return Err(napi::Error::from_reason(
                "position must have exactly 3 components [x, y, z]",
            ));
        }
        if rotation.len() != 4 {
            return Err(napi::Error::from_reason(
                "rotation must have exactly 4 components [x, y, z, w]",
            ));
        }

        let obj = self.scene_objects.get_mut(&id_num).ok_or_else(|| {
            napi::Error::from_reason(format!("Scene object not found for ID: \"{id}\""))
        })?;

        let pos = glam::Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32);
        // Normalize the quaternion to ensure a pure rotation with no scaling artefacts.
        let quat = glam::Quat::from_xyzw(
            rotation[0] as f32,
            rotation[1] as f32,
            rotation[2] as f32,
            rotation[3] as f32,
        )
        .normalize();
        obj.transform = glam::Mat4::from_rotation_translation(quat, pos);

        Ok(())
    }

    /// Step 5 / 6: Set the camera for the next `renderRaw` call.
    ///
    /// Uses a right-handed coordinate system (Three.js / OpenGL convention):
    /// * +X is right
    /// * +Y is up
    /// * +Z points **toward** the viewer (out of the screen)
    ///
    /// # Arguments
    /// * `position`    – `[x, y, z]` eye position in world space.
    /// * `target`      – `[x, y, z]` look-at point in world space.
    /// * `fov_degrees` – Vertical field-of-view in degrees.
    #[napi]
    pub fn set_camera(
        &mut self,
        position: Vec<f64>,
        target: Vec<f64>,
        fov_degrees: f64,
    ) -> napi::Result<()> {
        if position.len() != 3 {
            return Err(napi::Error::from_reason(
                "position must have exactly 3 components [x, y, z]",
            ));
        }
        if target.len() != 3 {
            return Err(napi::Error::from_reason(
                "target must have exactly 3 components [x, y, z]",
            ));
        }
        self.camera = CameraState {
            position: glam::Vec3::new(
                position[0] as f32,
                position[1] as f32,
                position[2] as f32,
            ),
            target: glam::Vec3::new(target[0] as f32, target[1] as f32, target[2] as f32),
            fov_deg: fov_degrees as f32,
        };
        Ok(())
    }

    /// Step 6: Add a directional light to the scene.
    ///
    /// # Arguments
    /// * `direction` – `[x, y, z]` direction the light travels (from source toward scene).
    ///                 Matches Three.js `DirectionalLight` convention.
    /// * `intensity` – Light intensity scalar (recommended range `0.0`–`1.0`).
    #[napi]
    pub fn add_directional_light(
        &mut self,
        direction: Vec<f64>,
        intensity: f64,
    ) -> napi::Result<()> {
        if direction.len() != 3 {
            return Err(napi::Error::from_reason(
                "direction must have exactly 3 components [x, y, z]",
            ));
        }
        let dir = glam::Vec3::new(
            direction[0] as f32,
            direction[1] as f32,
            direction[2] as f32,
        );
        // Normalise to avoid scaling artefacts in the shader.
        let dir = if dir.length_squared() > 0.0 {
            dir.normalize()
        } else {
            glam::Vec3::NEG_Y // safe fallback: light points downward
        };
        self.directional_lights.push(DirectionalLightState {
            direction: dir,
            intensity: intensity as f32,
        });
        Ok(())
    }

    /// Step 4 / 5: Render the current scene and return the raw RGBA pixel data.
    ///
    /// Creates an off-screen `wgpu::Texture` (RENDER_ATTACHMENT | COPY_SRC), performs a
    /// render pass that clears to bright red and draws any geometry in the scene using
    /// the compiled WGSL shader pipeline, copies the texture into a CPU-visible buffer,
    /// maps it synchronously, strips row-alignment padding, and returns the pixel bytes.
    ///
    /// # Arguments
    /// * `camera_id` – Reserved for future use; pass any string (e.g. `"default"`).
    ///
    /// # Returns
    /// A `Uint8Array` of length `width * height * 4` in RGBA byte order.
    #[napi]
    pub fn render_raw(&self, _camera_id: String) -> napi::Result<Uint8Array> {
        // ── 1. Create the colour render target texture ───────────────────────────
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_target"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // ── 2. Create the depth texture ──────────────────────────────────────────
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_target"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ── 3. Create the CPU-readable output buffer ─────────────────────────────
        // wgpu requires each row of texels to be padded to COPY_BYTES_PER_ROW_ALIGNMENT.
        let bytes_per_pixel = 4u32; // RGBA8
        let unpadded_bytes_per_row = self.width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
        let buffer_size = (padded_bytes_per_row * self.height) as u64;

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── 4. Build uniform data for camera and light ───────────────────────────
        let aspect = self.width as f32 / self.height as f32;
        let fov_rad = self.camera.fov_deg.to_radians();
        // Right-handed view matrix (Three.js / OpenGL convention: +Y up, +Z toward viewer).
        let view = glam::Mat4::look_at_rh(
            self.camera.position,
            self.camera.target,
            glam::Vec3::Y,
        );
        // Right-handed perspective with OpenGL NDC depth range [-1, 1].
        let proj = glam::Mat4::perspective_rh_gl(fov_rad, aspect, 0.1, 100.0);
        let view_proj = proj * view;
        let camera_uniform_bytes = mat4_to_bytes(&view_proj);

        // Use the first directional light if present; otherwise a neutral default.
        let (light_dir, light_intensity) = self
            .directional_lights
            .first()
            .map(|l| (l.direction, l.intensity))
            .unwrap_or((glam::Vec3::new(0.0, -1.0, 0.0), 0.0));
        let light_uniform_bytes = light_to_bytes(light_dir, light_intensity);

        // ── 5. Build cube geometry buffers (shared by all cube objects) ───────────
        let (cube_vertex_data, cube_index_data) = cube_geometry();
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cube_vertices"),
                contents: &cube_vertex_data,
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cube_indices"),
                contents: &cube_index_data,
                usage: wgpu::BufferUsages::INDEX,
            });
        let index_count = (cube_index_data.len() / 2) as u32; // u16 indices

        // ── 6. Camera uniform buffer (shared across all draw calls) ───────────────
        let camera_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("camera_uniform"),
                contents: &camera_uniform_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Light uniform buffer (shared across all draw calls).
        let light_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("light_uniform"),
                contents: &light_uniform_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // ── 7. Record commands ────────────────────────────────────────────────────
        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_encoder"),
                });

        {
            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            // Draw each scene object using the compiled shader pipeline.
            if !self.scene_objects.is_empty() {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );

                for obj in self.scene_objects.values() {
                    // Only cube geometry is supported in this step; skip unknown types.
                    if obj.primitive_type != "cube" {
                        continue;
                    }

                    // Compute the normal matrix: transpose(inverse(model)).
                    // For uniform-scale transforms this equals the model matrix.
                    let normal_mat = obj.transform.inverse().transpose();
                    let model_uniform_bytes = model_to_bytes(&obj.transform, &normal_mat);

                    let model_buffer =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("model_uniform"),
                                contents: &model_uniform_bytes,
                                usage: wgpu::BufferUsages::UNIFORM,
                            });

                    let bind_group =
                        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("main_bind_group"),
                            layout: &self.bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: camera_buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: model_buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: light_buffer.as_entire_binding(),
                                },
                            ],
                        });

                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.draw_indexed(0..index_count, 0, 0..1);
                }
            }
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        // ── 8. Submit and wait for completion ────────────────────────────────────
        let submission = self.queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::MAX),
            })
            .map_err(|e| napi::Error::from_reason(format!("Device poll failed: {e:?}")))?;

        receiver
            .recv()
            .map_err(|_| napi::Error::from_reason("Buffer map channel closed unexpectedly"))?
            .map_err(|e| napi::Error::from_reason(format!("Buffer map failed: {e}")))?;

        // ── 9. Read back, strip row padding, and return ──────────────────────────
        let padded_data = buffer_slice.get_mapped_range();
        let result: Vec<u8> = if padded_bytes_per_row == unpadded_bytes_per_row {
            padded_data.to_vec()
        } else {
            padded_data
                .chunks(padded_bytes_per_row as usize)
                .flat_map(|row| row[..unpadded_bytes_per_row as usize].iter().copied())
                .collect()
        };
        drop(padded_data);
        output_buffer.unmap();

        Ok(Uint8Array::new(result))
    }
}

/// Initialize a headless wgpu device.
async fn init_wgpu(enable_gpu: bool) -> Result<(wgpu::Device, wgpu::Queue), String> {
    // On Linux without a display server we need the Vulkan backend.
    // `WGPU_BACKEND` env-var can override this at runtime.
    let backends = std::env::var("WGPU_BACKEND")
        .ok()
        .map(|s| wgpu::Backends::from_comma_list(&s))
        .unwrap_or_else(|| wgpu::Backends::VULKAN | wgpu::Backends::GL);

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });

    // Power preference: low-power / software when GPU is disabled.
    let power_preference = if enable_gpu {
        wgpu::PowerPreference::HighPerformance
    } else {
        wgpu::PowerPreference::None
    };

    // `force_fallback_adapter` forces wgpu to use a software renderer.
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference,
            force_fallback_adapter: !enable_gpu,
            compatible_surface: None, // headless – no window surface
        })
        .await
        .map_err(|e| format!("No suitable wgpu adapter found (ensure a Vulkan driver such as lavapipe is installed): {e}"))?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("nexus-render-core"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            },
        )
        .await
        .map_err(|e| format!("Failed to create wgpu device: {e}"))?;

    Ok((device, queue))
}

// ── Pipeline builder ──────────────────────────────────────────────────────────

/// Build the main render pipeline from the embedded WGSL shader.
///
/// Returns `(RenderPipeline, BindGroupLayout)`.
fn build_render_pipeline(
    device: &wgpu::Device,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("main_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    // Three uniform bindings: camera, model, light.
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("main_bgl"),
        entries: &[
            // Binding 0: camera View-Projection matrix (vertex stage)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Binding 1: per-object model + normal matrices (vertex stage)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Binding 2: directional light (fragment stage)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("main_pl"),
        bind_group_layouts: &[&bgl],
        immediate_size: 0,
    });

    // Vertex layout: position (3×f32) + normal (3×f32) = 24 bytes/vertex.
    let vertex_attributes = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 12,
            shader_location: 1,
        },
    ];
    let vertex_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: 24,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &vertex_attributes,
    };

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("main_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_buffer_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    (pipeline, bgl)
}

// ── Geometry ──────────────────────────────────────────────────────────────────

/// Returns interleaved vertex data (position + normal, 6×f32 per vertex) and
/// u16 index data for a unit cube centred at the origin.
///
/// Winding is counter-clockwise (CCW) when viewed from outside the cube, which
/// is the front-face convention used by the render pipeline.
fn cube_geometry() -> (Vec<u8>, Vec<u8>) {
    // 24 vertices – 4 per face, 6 faces.
    // Each vertex: position [f32; 3] + normal [f32; 3].
    #[rustfmt::skip]
    let verts: &[[f32; 6]] = &[
        // +X face  (normal 1, 0, 0)
        [ 0.5,  0.5,  0.5,   1.0,  0.0,  0.0],
        [ 0.5, -0.5,  0.5,   1.0,  0.0,  0.0],
        [ 0.5, -0.5, -0.5,   1.0,  0.0,  0.0],
        [ 0.5,  0.5, -0.5,   1.0,  0.0,  0.0],
        // -X face  (normal -1, 0, 0)
        [-0.5,  0.5, -0.5,  -1.0,  0.0,  0.0],
        [-0.5, -0.5, -0.5,  -1.0,  0.0,  0.0],
        [-0.5, -0.5,  0.5,  -1.0,  0.0,  0.0],
        [-0.5,  0.5,  0.5,  -1.0,  0.0,  0.0],
        // +Y face  (normal 0, 1, 0)
        [-0.5,  0.5,  0.5,   0.0,  1.0,  0.0],
        [ 0.5,  0.5,  0.5,   0.0,  1.0,  0.0],
        [ 0.5,  0.5, -0.5,   0.0,  1.0,  0.0],
        [-0.5,  0.5, -0.5,   0.0,  1.0,  0.0],
        // -Y face  (normal 0, -1, 0)
        [-0.5, -0.5, -0.5,   0.0, -1.0,  0.0],
        [ 0.5, -0.5, -0.5,   0.0, -1.0,  0.0],
        [ 0.5, -0.5,  0.5,   0.0, -1.0,  0.0],
        [-0.5, -0.5,  0.5,   0.0, -1.0,  0.0],
        // +Z face  (normal 0, 0, 1)  – visible when camera is on +Z side
        [-0.5,  0.5,  0.5,   0.0,  0.0,  1.0],
        [-0.5, -0.5,  0.5,   0.0,  0.0,  1.0],
        [ 0.5, -0.5,  0.5,   0.0,  0.0,  1.0],
        [ 0.5,  0.5,  0.5,   0.0,  0.0,  1.0],
        // -Z face  (normal 0, 0, -1)
        [ 0.5,  0.5, -0.5,   0.0,  0.0, -1.0],
        [ 0.5, -0.5, -0.5,   0.0,  0.0, -1.0],
        [-0.5, -0.5, -0.5,   0.0,  0.0, -1.0],
        [-0.5,  0.5, -0.5,   0.0,  0.0, -1.0],
    ];

    let vertex_bytes: Vec<u8> = verts
        .iter()
        .flat_map(|v| v.iter().flat_map(|f| f.to_le_bytes()))
        .collect();

    // 6 faces × 2 triangles × 3 indices = 36 u16 indices.
    // Each face uses vertices 0-3 with the pattern (0,1,2) and (0,2,3).
    let index_bytes: Vec<u8> = (0u16..6)
        .flat_map(|face| {
            let b = face * 4;
            [b, b + 1, b + 2, b, b + 2, b + 3]
        })
        .flat_map(|i| i.to_le_bytes())
        .collect();

    (vertex_bytes, index_bytes)
}

// ── Uniform helpers ───────────────────────────────────────────────────────────

/// Serialise a `glam::Mat4` to 64 little-endian bytes (column-major, matching WGSL).
fn mat4_to_bytes(m: &glam::Mat4) -> [u8; 64] {
    let mut out = [0u8; 64];
    for (i, f) in m.to_cols_array().iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&f.to_le_bytes());
    }
    out
}

/// Serialise the model matrix and its normal matrix into 128 little-endian bytes.
fn model_to_bytes(model: &glam::Mat4, normal_mat: &glam::Mat4) -> [u8; 128] {
    let mut out = [0u8; 128];
    out[..64].copy_from_slice(&mat4_to_bytes(model));
    out[64..].copy_from_slice(&mat4_to_bytes(normal_mat));
    out
}

/// Serialise a directional light into 16 little-endian bytes.
///
/// WGSL layout: `direction: vec3<f32>` (12 bytes) + `intensity: f32` (4 bytes).
fn light_to_bytes(direction: glam::Vec3, intensity: f32) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&direction.x.to_le_bytes());
    out[4..8].copy_from_slice(&direction.y.to_le_bytes());
    out[8..12].copy_from_slice(&direction.z.to_le_bytes());
    out[12..16].copy_from_slice(&intensity.to_le_bytes());
    out
}
