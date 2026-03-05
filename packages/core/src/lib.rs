#![deny(clippy::all)]

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

/// Step 2: Headless wgpu RenderEngine.
///
/// Initializes a wgpu Instance, Adapter, Device, and Queue in headless mode.
/// Falls back to software rendering (lavapipe) when no physical GPU is available.
#[napi]
pub struct RenderEngine {
    width: u32,
    height: u32,
    #[allow(dead_code)]
    device: wgpu::Device,
    #[allow(dead_code)]
    queue: wgpu::Queue,
}

#[napi]
impl RenderEngine {
    /// Create a new headless RenderEngine.
    ///
    /// # Arguments
    /// * `options.width`     – Render target width in pixels.
    /// * `options.height`    – Render target height in pixels.
    /// * `options.enableGPU` – When `true` prefer a hardware GPU; when `false`
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
