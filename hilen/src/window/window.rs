use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    mpsc::Receiver,
};

use anyhow::{Context, Result, bail};
#[cfg(any(desktop, wasm))]
use log::error;
use log::{info, warn};
use plat::Platform;
#[cfg(linux)]
use wgpu::InstanceFlags;
use wgpu::{
    Adapter, Backends, CompositeAlphaMode, Device, DeviceDescriptor, ExperimentalFeatures, Features,
    Instance, InstanceDescriptor, Limits, MemoryHints, PowerPreference, PresentMode, Queue,
    RequestAdapterOptions, SurfaceColorSpace, SurfaceConfiguration, TextureUsages, Trace,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

#[cfg(desktop)]
use crate::window::icon::apply_icon;
use crate::{
    deps::hreads::on_main,
    gm::{
        LossyConvert,
        color::Color,
        flat::{Point, Size},
    },
    window::{
        Screenshot, UserEvent,
        app_handler::AppHandler,
        screen::Screen,
        state::{State, surface_texture_format},
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
/// The browser surface is render attachment only, and the android
/// swapchain rejects the copy usage on some devices. Screenshots there
/// read the scene texture instead of copying the surface.
pub(crate) const SURFACE_COPY: bool = !Platform::WASM && !Platform::ANDROID;

pub struct Window {
    pub state: State,

    pub(crate) instance: Instance,
    pub(crate) adapter:  Adapter,
    pub(crate) device:   Device,
    pub(crate) queue:    Queue,

    pub(crate) screen: Screen,

    pub(crate) title_set: bool,

    /// A label kept in front of the frame stats in the title, so a UI test
    /// under human review still shows the frame time like an app does.
    pub(crate) title_prefix: Option<String>,

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
            Screen::Windowed { size, .. } => (size.width, size.height).into(),
            #[cfg(not_wasm)]
            Screen::Headless { size } => (size.width, size.height).into(),
        }
    }

    /// Store the inner size a window event reported. Every later size
    /// query reads it, see `Screen::Windowed`.
    pub(crate) fn record_inner_size(&mut self, size: PhysicalSize<u32>) {
        match &mut self.screen {
            Screen::Windowed { size: recorded, .. } => *recorded = Size::new(size.width, size.height),
            #[cfg(not_wasm)]
            Screen::Headless { .. } => {}
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

    pub(crate) fn set_clear_color(color: impl Into<Color>) {
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
        }

        required_limits
    }

    /// Windows asks for DX12 alone. With every backend enabled wgpu picks
    /// Vulkan there, and the Intel driver for Gen9 integrated GPUs faults
    /// inside `vkCreateDevice`, which kills the process with no message.
    /// Android asks for Vulkan alone. With GL and Vulkan both enabled they
    /// race for the one `ANativeWindow`, the loser gets
    /// `ERROR_NATIVE_WINDOW_IN_USE_KHR` and wgpu-hal panics instead of
    /// skipping that backend. `WGPU_BACKEND` still overrides the choice on
    /// any platform.
    ///
    /// WSL allows a non conformant adapter. The only Vulkan driver that
    /// reaches the Windows GPU there is Mesa's Direct3D 12 one, and it
    /// reports conformance version 0, which wgpu hides by default. Without
    /// the flag wgpu falls back to the CPU lavapipe and draws every frame
    /// in software on every core, see docs/wsl.md.
    fn instance() -> Instance {
        let mut descriptor = InstanceDescriptor::new_without_display_handle();

        if Platform::WINDOWS {
            descriptor.backends = Backends::DX12;
        }

        if Platform::ANDROID {
            descriptor.backends = Backends::VULKAN;
        }

        #[cfg(linux)]
        if crate::window::wsl::active() {
            descriptor.flags |= InstanceFlags::ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER;
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

    /// The surface on `window` and an adapter that can present to it.
    async fn adapter_on(
        instance: Instance,
        window: Arc<winit::window::Window>,
    ) -> Result<(Instance, wgpu::Surface<'static>, Adapter)> {
        let surface = instance.create_surface(window).context("Failed to create surface")?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference:       PowerPreference::HighPerformance,
                force_fallback_adapter: false,

                compatible_surface:  Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .context("Could not get a GPU adapter")?;

        Ok((instance, surface, adapter))
    }

    /// The instance a browser gets. WebGPU when the page has it and it
    /// hands out an adapter, WebGL otherwise. A browser can expose
    /// `navigator.gpu` and still answer the adapter request with null,
    /// `WebKit` does on the iOS simulator and in Lockdown mode, and wgpu
    /// takes WebGPU whenever the property exists and never falls back by
    /// itself. The probe asks for an adapter with no surface, because a
    /// canvas keeps the first context kind it is given, so a WebGPU
    /// surface tried first would leave no canvas for WebGL. `hilen_webgl`
    /// in the page query forces WebGL, to check that path in a browser
    /// that has WebGPU. Without the `webgl` feature the GL instance is
    /// WebGPU again and its adapter error is the one reported.
    #[cfg(wasm)]
    async fn browser_instance() -> Instance {
        if crate::web::query_flag("hilen_webgl") {
            info!("WebGL forced by the page query");
            return Self::webgl_instance();
        }

        if !crate::web::has_webgpu() {
            return Self::webgl_instance();
        }

        let instance = Self::instance();

        let probe = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference:       PowerPreference::HighPerformance,
                force_fallback_adapter: false,

                compatible_surface:  None,
                apply_limit_buckets: false,
            })
            .await;

        match probe {
            Ok(_) => instance,
            Err(err) => {
                warn!("WebGPU gave no adapter, using WebGL: {err}");
                Self::webgl_instance()
            }
        }
    }

    #[cfg(wasm)]
    fn webgl_instance() -> Instance {
        let mut descriptor = InstanceDescriptor::new_without_display_handle();
        descriptor.backends = Backends::GL;
        Instance::new(descriptor)
    }

    pub(crate) async fn start_internal(
        size: PhysicalSize<u32>,
        window: winit::window::Window,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<()> {
        let winit_window = Arc::new(window);

        #[cfg(wasm)]
        let instance = Self::browser_instance().await;
        #[cfg(not_wasm)]
        let instance = Self::instance();

        let (instance, surface, adapter) = Self::adapter_on(instance, winit_window.clone()).await?;

        let info = adapter.get_info();

        info!("Backend: {}", info.backend);

        // Everything down the line asks for the render format, so the
        // browser canvas format resolves first.
        #[cfg(wasm)]
        crate::window::state::web_formats::resolve(&surface, &adapter);

        let (device, queue) = Self::request_device(&adapter).await?;

        // A browser reports a failed pipeline or a bad draw through this
        // event and otherwise carries on, every later frame of the page
        // black. WebKit refused the text shader that way. Native wgpu
        // panics on the same errors, so the page treats them as fatal
        // too, and drops the canvas so the fallback content shows.
        #[cfg(wasm)]
        device.on_uncaptured_error(Arc::new(|err| {
            error!("Fatal GPU error: {err}");
            crate::web::drop_canvas();
        }));

        // Shadowing would keep the adapter probe surface alive to the end of
        // the function. Android allows one producer per native window, so it
        // must go before `Surface::new` connects its own, or Vulkan fails
        // with `ERROR_NATIVE_WINDOW_IN_USE_KHR`.
        drop(surface);

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

        let inner = winit_window.inner_size();

        let window = Self {
            state: State::default(),
            instance,
            adapter,
            device,
            queue,
            screen: Screen::Windowed {
                winit_window,
                surface,
                size: Size::new(inner.width, inner.height),
            },
            #[cfg(desktop)]
            is_resizing: false,
            title_set: false,
            title_prefix: None,
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
            title_prefix: None,
        })
    }

    /// Put `prefix` in front of the frame stats in the title. Unlike
    /// `set_title` this keeps the stats, so a human mode test prompt still
    /// shows the frame time.
    pub fn set_title_prefix(prefix: impl Into<String>) {
        let prefix = prefix.into();
        on_main(move || {
            let window = Self::current();
            window.title_prefix = Some(prefix.clone());

            // The browser has no per frame title, the prefix is the title.
            #[cfg(wasm)]
            web_sys::window()
                .expect("Failed to get browser window")
                .document()
                .expect("Failed to get browser document")
                .set_title(&prefix);

            #[cfg(not_wasm)]
            if let Some(winit) = Self::winit_window()
                && Platform::DESKTOP
            {
                winit.set_title(&prefix);
            }
        });
    }

    /// The title text for the frame stats, with the prefix in front when
    /// one is set.
    pub(crate) fn stats_title(stats: &str) -> String {
        match &Self::current().title_prefix {
            Some(prefix) => format!("{prefix} | {stats}"),
            None => stats.to_string(),
        }
    }

    pub fn set_title(title: impl Into<String>) {
        let title = title.into();
        on_main(move || {
            Self::current().title_set = true;

            // winit's web backend writes the title into the canvas alt
            // attribute, which nobody sees. The tab title is the visible
            // one, and human mode prompts live in the title.
            #[cfg(wasm)]
            web_sys::window()
                .expect("Failed to get browser window")
                .document()
                .expect("Failed to get browser document")
                .set_title(&title);

            #[cfg(not_wasm)]
            if let Some(window) = Self::winit_window() {
                if Platform::DESKTOP {
                    window.set_title(&title);
                } else {
                    warn!("set_title is not supported on this platform");
                }
            }
        });
    }

    /// The icon the OS shows for the running process, from encoded image
    /// bytes such as a PNG. On macOS this is the Dock icon, which a bare
    /// binary outside an app bundle otherwise lacks. On Windows and Linux
    /// it is the window and taskbar icon. Phones and the browser take
    /// the icon from the bundle or the page, so the call does nothing there.
    pub fn set_icon(data: &'static [u8]) {
        #[cfg(desktop)]
        on_main(move || {
            if let Err(err) = apply_icon(data) {
                error!("Failed to set the app icon: {err}");
            }
        });
        #[cfg(not(desktop))]
        log::debug!(
            "The {} byte app icon is not applied here, the bundle or the page carries it",
            data.len()
        );
    }

    #[cfg(desktop)]
    pub(crate) fn set_size(&mut self, size: impl Into<Size<u32>>) {
        let size = size.into();

        let current_size: Size<u32> = Window::inner_size().lossy_convert();

        if size == current_size {
            return;
        }

        if let Screen::Headless { size: headless_size } = &mut self.screen {
            *headless_size = size;
            State::resize();
            return;
        }

        self.request_inner_size(PhysicalSize::new(size.width, size.height));
    }

    /// Ask winit for a new inner size and keep `is_resizing` true to the
    /// winit contract. `Some` means the platform applied it now and sends no
    /// `Resized`, Wayland does that, so the surface is reconfigured here.
    /// `None` means a `Resized` follows and clears the flag, X11 and the rest.
    /// Waiting for a `Resized` that never comes skips every frame and the
    /// window never shows.
    #[cfg(desktop)]
    pub(crate) fn request_inner_size(&mut self, size: impl Into<winit::dpi::Size>) {
        let Some(window) = self.screen.winit_window() else {
            return;
        };
        let size: winit::dpi::Size = size.into();
        if size.to_physical::<u32>(window.scale_factor()) == window.inner_size() {
            return;
        }
        match window.request_inner_size(size) {
            Some(applied) => {
                self.record_inner_size(applied);
                State::resize();
            }
            None => self.is_resizing = true,
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

    #[cfg(feature = "level")]
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
        format:       surface_texture_format(),
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
