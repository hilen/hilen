use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    mpsc::Receiver,
};

use anyhow::{Context, Result, bail};
use hreads::on_main;
use log::{info, warn};
use plat::Platform;
use wgpu::{
    Adapter, Backends, CompositeAlphaMode, Device, DeviceDescriptor, ExperimentalFeatures, Features,
    Instance, InstanceDescriptor, Limits, MemoryHints, PowerPreference, PresentMode, Queue,
    RequestAdapterOptions, SurfaceColorSpace, SurfaceConfiguration, TextureUsages, Trace,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use crate::{
    gm::{
        LossyConvert,
        color::Color,
        flat::{Point, Size},
    },
    window::{
        Screenshot, UserEvent,
        app_handler::AppHandler,
        screen::Screen,
        state::{SURFACE_TEXTURE_FORMAT, State},
        surface::Surface,
    },
};

static VSYNC: AtomicBool = AtomicBool::new(true);
static MAX_FRAME_LATENCY: AtomicU32 = AtomicU32::new(2);
static QUIT_ON_ESCAPE: AtomicBool = AtomicBool::new(false);
static RENDER_FRAME: AtomicU64 = AtomicU64::new(0);
/// Mirrors `Screen::Headless` so any thread can check it. Set once at
/// startup, never changes.
static HEADLESS: AtomicBool = AtomicBool::new(false);
/// Doesn't work on some Androids
pub(crate) const SUPPORT_SCREENSHOT: bool = !Platform::ANDROID;

/// The browser surface is render attachment only, screenshots there
/// read the scene texture instead of copying the surface.
pub(crate) const SURFACE_COPY: bool = SUPPORT_SCREENSHOT && !Platform::WASM;

pub struct Window {
    pub state: State,

    pub(crate) instance: Instance,
    pub(crate) adapter:  Adapter,
    pub(crate) device:   Device,
    pub(crate) queue:    Queue,

    pub(crate) screen: Screen,

    pub(crate) title_set: bool,

    #[cfg(desktop)]
    pub(crate) is_resizing: bool,
}

impl Window {
    pub fn current() -> &'static mut Self {
        AppHandler::window()
    }

    pub(crate) fn device() -> &'static Device {
        &Self::current().device
    }

    pub(crate) fn queue() -> &'static Queue {
        &Self::current().queue
    }

    /// Rendering goes to an offscreen texture, there is no window and no
    /// display. Decided at startup. Callable from any thread.
    pub fn headless() -> bool {
        HEADLESS.load(Ordering::Relaxed)
    }

    #[cfg(desktop)]
    pub(crate) fn is_resizing() -> bool {
        Self::current().is_resizing
    }

    pub(crate) fn winit_window() -> Option<&'static winit::window::Window> {
        Self::current().screen.winit_window()
    }

    /// The OS theme. `None` in headless mode or when the OS does not
    /// report one.
    pub(crate) fn system_theme() -> Option<winit::window::Theme> {
        Self::winit_window()?.theme()
    }

    pub fn inner_size() -> Size {
        match &Self::current().screen {
            Screen::Windowed { winit_window, .. } => {
                let size = winit_window.inner_size();
                (size.width, size.height).into()
            }
            #[cfg(not_wasm)]
            Screen::Headless { size } => (size.width, size.height).into(),
        }
    }

    pub(crate) fn outer_size() -> Size {
        match &Self::current().screen {
            Screen::Windowed { winit_window, .. } => {
                let size = winit_window.outer_size();
                (size.width, size.height).into()
            }
            #[cfg(not_wasm)]
            Screen::Headless { size } => (size.width, size.height).into(),
        }
    }

    pub fn render_size() -> Size {
        if Platform::IOS {
            Window::outer_size()
        } else {
            Window::inner_size()
        }
    }

    pub(crate) fn inner_position() -> Point {
        let Some(window) = Self::winit_window() else {
            return Point::default();
        };
        let pos = window.inner_position().unwrap_or_default();
        (pos.x, pos.y).into()
    }

    pub(crate) fn outer_position() -> Point {
        let Some(window) = Self::winit_window() else {
            return Point::default();
        };
        let pos = window.outer_position().unwrap_or_default();
        (pos.x, pos.y).into()
    }

    pub(crate) fn screen_scale() -> f32 {
        let Some(window) = Self::winit_window() else {
            return 1.0;
        };
        window.scale_factor().lossy_convert()
    }

    pub fn set_clear_color(color: impl Into<Color>) {
        Self::current().state.clear_color = color.into();
    }

    /// The test harness paints its own background and has to put this back, or
    /// the app keeps the harness colour after a run.
    pub(crate) fn clear_color() -> Color {
        Self::current().state.clear_color
    }

    pub(crate) fn close() {
        on_main(AppHandler::close);
    }

    /// The limits we ask the GPU for. Based on what the adapter reports it can
    /// do, so the request never exceeds the adapter and gets rejected. The iOS
    /// Simulator exposes lower Metal limits than `Limits::default`, so asking
    /// for the defaults there failed device creation.
    fn required_limits(adapter_limits: Limits) -> Limits {
        let mut required_limits = if Platform::WASM {
            let mut limits = Limits::downlevel_webgl2_defaults();
            limits.max_texture_dimension_1d = 8192;
            limits.max_texture_dimension_2d = 8192;
            limits
        } else {
            adapter_limits
        };

        if Platform::IOS {
            required_limits.max_color_attachments = 4;
        } else if Platform::ANDROID {
            // TODO:
            required_limits.max_compute_invocations_per_workgroup = 0;
            required_limits.max_compute_workgroups_per_dimension = 0;
            required_limits.max_compute_workgroup_storage_size = 0;
            required_limits.max_compute_workgroup_size_x = 0;
            required_limits.max_compute_workgroup_size_y = 0;
            required_limits.max_compute_workgroup_size_z = 0;
            required_limits.max_storage_buffer_binding_size = 0;
            required_limits.max_storage_textures_per_shader_stage = 0;
            required_limits.max_storage_buffers_per_shader_stage = 0;
            required_limits.max_dynamic_storage_buffers_per_pipeline_layout = 0;
            required_limits.max_texture_dimension_3d = 1024;
            required_limits.max_texture_dimension_2d = 4096;
            required_limits.max_texture_dimension_1d = 4096;
        }

        required_limits
    }

    /// Windows asks for DX12 alone. With every backend enabled wgpu picks
    /// Vulkan there, and the Intel driver for Gen9 integrated GPUs faults
    /// inside `vkCreateDevice`, which kills the process with no message.
    /// `WGPU_BACKEND` still overrides the choice on any platform.
    fn instance() -> Instance {
        let mut descriptor = InstanceDescriptor::new_without_display_handle();

        if Platform::WINDOWS {
            descriptor.backends = Backends::DX12;
        }

        Instance::new(descriptor.with_env())
    }

    async fn request_device(adapter: &Adapter) -> Result<(Device, Queue)> {
        let required_limits = Self::required_limits(adapter.limits());

        #[cfg(feature = "bench")]
        let required_features = {
            assert!(
                adapter.features().contains(Features::TIMESTAMP_QUERY),
                "bench feature needs GPU TIMESTAMP_QUERY support, this adapter lacks it"
            );
            Features::TIMESTAMP_QUERY
        };
        #[cfg(not(feature = "bench"))]
        let required_features = Features::empty();

        adapter
            .request_device(&DeviceDescriptor {
                required_features,
                // Doesn't work on some Androids
                // required_features: Features::POLYGON_MODE_LINE, // | Features::POLYGON_MODE_POINT,
                required_limits,
                label: None,
                memory_hints: MemoryHints::Performance,
                trace: Trace::default(),
                experimental_features: ExperimentalFeatures::default(),
            })
            .await
            .context("Failed to request GPU device")
    }

    pub(crate) async fn start_internal(
        size: PhysicalSize<u32>,
        window: winit::window::Window,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<()> {
        let winit_window = Arc::new(window);

        let instance = Self::instance();
        let surface = instance
            .create_surface(winit_window.clone())
            .context("Failed to create surface")?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference:       PowerPreference::HighPerformance,
                force_fallback_adapter: false,

                compatible_surface:  Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .context("Could not get a GPU adapter")?;

        let info = adapter.get_info();

        info!("Backend: {}", info.backend);

        let (device, queue) = Self::request_device(&adapter).await?;

        let surface = if size.width != 0 && size.height != 0 {
            Surface::new(
                &instance,
                &adapter,
                &device,
                surface_config_with_size((size.width, size.height)),
                winit_window.clone(),
            )
            .context("Failed to create surface")?
            .into()
        } else {
            None
        };

        let window = Self {
            state: State::default(),
            instance,
            adapter,
            device,
            queue,
            screen: Screen::Windowed {
                winit_window,
                surface,
            },
            #[cfg(desktop)]
            is_resizing: false,
            title_set: false,
        };

        if proxy.send_event(UserEvent::WindowReady(Box::new(window))).is_err() {
            bail!("Failed to send window event");
        }

        Ok(())
    }

    #[cfg(not_wasm)]
    pub(crate) async fn create_headless(size: Size<u32>) -> Result<Self> {
        let instance = Self::instance();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference:       PowerPreference::HighPerformance,
                force_fallback_adapter: false,

                compatible_surface:  None,
                apply_limit_buckets: false,
            })
            .await
            .context("Could not get a GPU adapter")?;

        let info = adapter.get_info();

        info!("Backend: {} (headless)", info.backend);

        HEADLESS.store(true, Ordering::Relaxed);

        let (device, queue) = Self::request_device(&adapter).await?;

        Ok(Self {
            state: State::default(),
            instance,
            adapter,
            device,
            queue,
            screen: Screen::Headless { size },
            #[cfg(desktop)]
            is_resizing: false,
            title_set: false,
        })
    }

    pub fn set_title(title: impl Into<String>) {
        let title = title.into();
        on_main(move || {
            Self::current().title_set = true;
            if let Some(window) = Self::winit_window() {
                if Platform::DESKTOP {
                    window.set_title(&title);
                } else {
                    warn!("set_title is not supported on this platform");
                }
            }
        });
    }

    #[cfg(desktop)]
    pub(crate) fn set_size(&mut self, size: impl Into<Size<u32>>) {
        let size = size.into();

        let current_size: Size<u32> = Window::inner_size().lossy_convert();

        if size == current_size {
            return;
        }

        match &mut self.screen {
            Screen::Windowed { winit_window, .. } => {
                self.is_resizing = true;
                let _ = winit_window.request_inner_size(PhysicalSize::new(size.width, size.height));
            }
            #[cfg(not_wasm)]
            Screen::Headless { size: headless_size } => {
                *headless_size = size;
                State::resize();
            }
        }
    }

    pub(crate) fn request_screenshot(&self) -> Receiver<Screenshot> {
        self.state.request_read_display()
    }

    pub fn fps(&self) -> f32 {
        self.state.frame_counter.fps
    }

    pub fn frame_time(&self) -> f32 {
        self.state.frame_counter.frame_time
    }

    /// CPU time of the last frame's update and render encoding. Not capped by
    /// vsync or the compositor - use for performance measurements.
    pub fn frame_work_time(&self) -> f32 {
        self.state.frame_work_time
    }

    /// GPU execution time of the last frame's render pass, from timestamp
    /// queries. Advisory: it carries clock and thermal noise the benchmark
    /// guard cannot catch.
    #[cfg(feature = "bench")]
    pub fn frame_gpu_time(&self) -> f32 {
        self.state.frame_gpu_time
    }

    pub(crate) fn frame_drawn(&self) -> u32 {
        self.state.frame_counter.frame_count
    }

    /// Close the app when Escape is pressed. Off by default.
    pub fn set_quit_on_escape(enable: bool) {
        QUIT_ON_ESCAPE.store(enable, Ordering::Relaxed);
    }

    pub(crate) fn quit_on_escape() -> bool {
        QUIT_ON_ESCAPE.load(Ordering::Relaxed)
    }

    /// Always enabled on mobile platforms. Takes effect on the next frame.
    pub fn set_vsync(enable: bool) {
        on_main(move || {
            VSYNC.store(enable, Ordering::Relaxed);
            Self::reconfigure_surface();
        });
    }

    /// How many frames the GPU is allowed to buffer ahead of the display.
    /// Default is 2 - lowest input latency. 3 renders faster but adds up to
    /// one frame of lag. Backends clamp unsupported values.
    pub fn set_max_frame_latency(latency: u32) {
        on_main(move || {
            MAX_FRAME_LATENCY.store(latency, Ordering::Relaxed);
            Self::reconfigure_surface();
        });
    }

    /// Index of the frame currently being rendered. Bumps once per rendered
    /// frame, before any draw code runs.
    pub fn render_frame() -> u64 {
        RENDER_FRAME.load(Ordering::Relaxed)
    }

    pub(crate) fn next_render_frame() {
        RENDER_FRAME.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn reconfigure_surface() {
        let window = Self::current();

        if let Screen::Windowed {
            surface: Some(surface),
            ..
        } = &window.screen
        {
            let size: Size<u32> = Self::render_size().lossy_convert();
            surface.presentable.configure(&window.device, &surface_config_with_size(size));
        }
    }

    pub(crate) fn display_refresh_rate() -> u32 {
        let Some(window) = Self::winit_window() else {
            return 60;
        };
        window.current_monitor().map_or(60, |monitor| {
            monitor.refresh_rate_millihertz().unwrap_or(60_000) / 1000
        })
    }
}

pub(crate) fn surface_config_with_size(size: impl Into<Size<u32>>) -> SurfaceConfiguration {
    let size: Size<u32> = size.into();

    SurfaceConfiguration {
        usage:        if SURFACE_COPY {
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC
        } else {
            TextureUsages::RENDER_ATTACHMENT
        },
        format:       SURFACE_TEXTURE_FORMAT,
        color_space:  SurfaceColorSpace::Auto,
        width:        size.width,
        height:       size.height,
        present_mode: if VSYNC.load(Ordering::Relaxed) || Platform::MOBILE {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        },
        alpha_mode:   CompositeAlphaMode::Auto,
        view_formats: vec![],

        desired_maximum_frame_latency: MAX_FRAME_LATENCY.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod test {
    use wgpu::Limits;

    use super::Window;

    // Regression: the device request must never exceed what the adapter
    // reports. The iOS Simulator exposes lower Metal limits than
    // Limits::default, and asking for the defaults there aborted the app
    // during GPU init.
    #[test]
    fn required_limits_stay_within_adapter() {
        let adapter = Limits::downlevel_defaults();
        let required = Window::required_limits(adapter.clone());

        assert!(required.max_texture_dimension_1d <= adapter.max_texture_dimension_1d);
        assert!(required.max_texture_dimension_2d <= adapter.max_texture_dimension_2d);
        assert!(required.max_texture_dimension_3d <= adapter.max_texture_dimension_3d);
        assert!(required.max_buffer_size <= adapter.max_buffer_size);
        assert!(required.max_color_attachments <= adapter.max_color_attachments);
    }
}
