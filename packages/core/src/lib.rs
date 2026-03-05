#![deny(clippy::all)]

use std::collections::HashMap;

use napi::bindgen_prelude::Uint8Array;
use napi_derive::napi;

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
    #[allow(dead_code)]
    primitive_type: String,
    /// The world-space transform matrix for this object.
    transform: glam::Mat4,
}

/// Step 2 / 3 / 4: Headless wgpu RenderEngine.
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

        Ok(Self {
            width: options.width,
            height: options.height,
            device,
            queue,
            scene_objects: HashMap::new(),
            next_id: 0,
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

        if position.len() < 3 {
            return Err(napi::Error::from_reason(
                "position must have at least 3 components [x, y, z]",
            ));
        }
        if rotation.len() < 4 {
            return Err(napi::Error::from_reason(
                "rotation must have at least 4 components [x, y, z, w]",
            ));
        }

        let obj = self.scene_objects.get_mut(&id_num).ok_or_else(|| {
            napi::Error::from_reason(format!("Scene object not found for ID: \"{id}\""))
        })?;

        let pos = glam::Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32);
        let quat = glam::Quat::from_xyzw(
            rotation[0] as f32,
            rotation[1] as f32,
            rotation[2] as f32,
            rotation[3] as f32,
        );
        obj.transform = glam::Mat4::from_rotation_translation(quat, pos);

        Ok(())
    }

    /// Step 4: Render the current scene and return the raw RGBA pixel data.
    ///
    /// Creates an off-screen `wgpu::Texture` (RENDER_ATTACHMENT | COPY_SRC), performs a
    /// render pass that clears to bright red, copies the texture into a CPU-visible buffer,
    /// maps it synchronously, strips row-alignment padding, and returns the pixel bytes.
    ///
    /// # Arguments
    /// * `camera_id` – Reserved for future use; pass any string (e.g. `"default"`).
    ///
    /// # Returns
    /// A `Uint8Array` of length `width * height * 4` in RGBA byte order.
    #[napi]
    pub fn render_raw(&self, _camera_id: String) -> napi::Result<Uint8Array> {
        // ── 1. Create the render target texture ─────────────────────────────────
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

        // ── 2. Create the CPU-readable output buffer ─────────────────────────────
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

        // ── 3. Record commands: clear pass + texture→buffer copy ─────────────────
        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_encoder"),
                });

        {
            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
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
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
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

        // ── 4. Submit and wait for completion ────────────────────────────────────
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

        // ── 5. Read back, strip row padding, and return ──────────────────────────
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
