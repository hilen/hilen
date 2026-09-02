use std::{
    cell::RefCell,
    sync::{
        OnceLock,
        mpsc::{Receiver, Sender, channel},
    },
};

#[cfg(feature = "bench")]
use anyhow::Result;
use log::{info, warn};
use plat::Platform;
use web_time::Instant;
use wgpu::{CurrentSurfaceTexture, TextureFormat};

use crate::{
    deps::refs::manage::DataManager,
    gm::{
        LossyConvert,
        color::{Color, GRAY_BLUE},
    },
    window::{
        Font, RenderFrame, Screenshot, Window, app_handler::AppHandler, frame_counter::FrameCounter,
        image::Texture, screen::Screen, surface::Surface, window::surface_config_with_size,
    },
};

type ReadDisplayRequest = Sender<Screenshot>;

enum RenderTarget {
    Skip,
    #[cfg(not_wasm)]
    Offscreen,
    Surface(wgpu::SurfaceTexture),
}

/// Start and end timestamp of the render pass.
#[cfg(feature = "bench")]
const GPU_TIMESTAMPS: u32 = 2;
#[cfg(feature = "bench")]
const GPU_TIMESTAMP_BYTES: u64 = GPU_TIMESTAMPS as u64 * 8;

/// GPU-side resources for one render pass timestamp pair. Created lazily on the
/// first benchmarked frame and reused after.
#[cfg(feature = "bench")]
struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve:   wgpu::Buffer,
    readback:  wgpu::Buffer,
}

// Plain Unorm, never an sRGB format. Color values are encoded sRGB end
// to end and the hardware must blend the written values as they are,
// the same math browsers, design tools and every UI stack use. An sRGB
// target would decode, blend in linear and encode back, which shifts
// every translucent and antialiased pixel away from the design.
// Rgba, not Bgra, android swapchains have no Bgra.
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
const SURFACE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
#[cfg(target_os = "android")]
const SURFACE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

/// The format every frame renders into. Fixed at compile time
/// everywhere but the browser, which picks its canvas format at
/// runtime.
pub fn surface_texture_format() -> TextureFormat {
    #[cfg(target_arch = "wasm32")]
    {
        web_formats::present_format()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SURFACE_TEXTURE_FORMAT
    }
}

/// Samples per pixel of the frame's render pass. 4 anti-aliases the
/// triangulated geometry the SDF pipelines cannot smooth, vector paths,
/// polygons and sprite cutouts. Every pipeline drawing into the pass
/// and the pass attachments must agree on this count. `HILEN_MSAA=1`
/// switches multisampling off, the A/B lever for benchmarks.
pub fn msaa_sample_count() -> u32 {
    static COUNT: OnceLock<u32> = OnceLock::new();
    *COUNT.get_or_init(|| {
        let count = std::env::var("HILEN_MSAA").map_or(4, |value| {
            value.parse().unwrap_or_else(|_| panic!("Invalid HILEN_MSAA value: {value}"))
        });
        assert!(
            matches!(count, 1 | 2 | 4),
            "HILEN_MSAA must be 1, 2 or 4, got {count}"
        );
        count
    })
}

/// The canvas format has to be what the browser prefers. Both allowed
/// formats are valid to configure, but rendering the non preferred one
/// makes Chrome convert every frame and makes Firefox present it with
/// red and blue swapped.
#[cfg(target_arch = "wasm32")]
pub(crate) mod web_formats {
    use std::sync::OnceLock;

    use wgpu::TextureFormat;

    static PRESENT: OnceLock<TextureFormat> = OnceLock::new();

    /// The first capability entry is the browser's preferred canvas
    /// format. Called during window creation, before anything that
    /// needs the format exists.
    pub(crate) fn resolve(surface: &wgpu::Surface, adapter: &wgpu::Adapter) {
        let format = surface.get_capabilities(adapter).formats[0];
        PRESENT.get_or_init(|| format);
    }

    pub(crate) fn present_format() -> TextureFormat {
        *PRESENT.get().expect("Surface format is not resolved yet")
    }
}

/// Where the surface cannot be copied, screenshots read the scene texture
/// instead, which therefore needs the copy usage.
const SCENE_TEXTURE_USAGE: wgpu::TextureUsages = if crate::window::SURFACE_COPY {
    wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::TEXTURE_BINDING)
} else {
    wgpu::TextureUsages::RENDER_ATTACHMENT
        .union(wgpu::TextureUsages::TEXTURE_BINDING)
        .union(wgpu::TextureUsages::COPY_SRC)
};

pub struct State {
    pub(crate) clear_color: Color,

    read_display_request:     RefCell<Option<ReadDisplayRequest>>,
    pub(crate) frame_counter: FrameCounter,

    offscreen_texture: Option<wgpu::Texture>,
    scene_texture:     Option<wgpu::Texture>,
    msaa_texture:      Option<wgpu::Texture>,
    depth_texture:     Option<Texture>,

    update_work: f32,

    /// CPU time of the last frame's update and render encoding, excluding the
    /// wait for a drawable and present. Unlike `frame_time` it is not capped
    /// by vsync or the compositor.
    pub(crate) frame_work_time: f32,

    /// GPU execution time of the last frame's render pass, in seconds.
    #[cfg(feature = "bench")]
    pub(crate) frame_gpu_time: f32,

    #[cfg(feature = "bench")]
    gpu_timer: Option<GpuTimer>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            clear_color:                              GRAY_BLUE,
            read_display_request:                     RefCell::default(),
            frame_counter:                            FrameCounter::default(),
            offscreen_texture:                        None,
            scene_texture:                            None,
            msaa_texture:                             None,
            depth_texture:                            None,
            update_work:                              0.0,
            frame_work_time:                          0.0,
            #[cfg(feature = "bench")]
            frame_gpu_time:                           0.0,
            #[cfg(feature = "bench")]
            gpu_timer:                                None,
        }
    }
}

impl State {
    pub(crate) fn resize() {
        let new_size = Window::render_size();

        if new_size.width == 0.0 || new_size.height == 0.0 {
            warn!("Zero size");
            return;
        }

        let window = Window::current();

        match &mut window.screen {
            Screen::Windowed {
                winit_window,
                surface,
            } => {
                if surface.is_none() {
                    *surface = Surface::new(
                        &window.instance,
                        &window.adapter,
                        &window.device,
                        surface_config_with_size((
                            new_size.width.lossy_convert(),
                            new_size.height.lossy_convert(),
                        )),
                        winit_window.clone(),
                    )
                    .expect("Failed to create surface")
                    .into();

                    info!("surface created");
                }

                surface.as_ref().unwrap().presentable.configure(
                    &window.device,
                    &surface_config_with_size((
                        new_size.width.lossy_convert(),
                        new_size.height.lossy_convert(),
                    )),
                );
            }
            #[cfg(not_wasm)]
            Screen::Headless { .. } => {}
        }

        let queue = Window::queue();

        for font in Font::storage_mut().values_mut() {
            font.brush.resize_view(new_size.width, new_size.height, queue);
        }

        AppHandler::current().te_window_events.resize(
            Window::inner_position(),
            Window::outer_position(),
            Window::inner_size(),
            Window::outer_size(),
        );

        #[cfg(desktop)]
        {
            window.is_resizing = false;
        }
    }

    pub(crate) fn update(&mut self) {
        #[cfg(desktop)]
        if Window::is_resizing() {
            return;
        }

        let work_started = Instant::now();
        AppHandler::current().te_window_events.update();
        self.update_work = work_started.elapsed().as_secs_f32();

        if self.frame_counter.update()
            && !Window::current().title_set
            && Platform::DESKTOP
            && let Some(window) = Window::winit_window()
        {
            window.set_title(&Window::stats_title(&format!(
                "{:.2}ms",
                self.frame_counter.frame_time * 1000.0
            )));
        }
    }

    /// Where the next frame goes. `Skip` means no frame can be rendered right
    /// now - the surface is missing, occluded or being recovered.
    fn acquire_render_target() -> RenderTarget {
        match &Window::current().screen {
            #[cfg(not_wasm)]
            Screen::Headless { .. } => RenderTarget::Offscreen,
            Screen::Windowed { surface: None, .. } => RenderTarget::Skip,
            Screen::Windowed {
                surface: Some(surface),
                ..
            } => match surface.presentable.get_current_texture() {
                CurrentSurfaceTexture::Success(tex) => RenderTarget::Surface(tex),
                CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => RenderTarget::Skip,
                CurrentSurfaceTexture::Suboptimal(tex) => {
                    // wgpu refuses to configure while an acquired texture is
                    // alive, and dropping it after the reconfigure panics in
                    // the destructor.
                    drop(tex);
                    Self::reconfigure_and_skip()
                }
                CurrentSurfaceTexture::Outdated => Self::reconfigure_and_skip(),
                CurrentSurfaceTexture::Lost => {
                    warn!("Surface is lost, recreating");
                    match &mut Window::current().screen {
                        Screen::Windowed { surface, .. } => *surface = None,
                        #[cfg(not_wasm)]
                        Screen::Headless { .. } => {}
                    }
                    Self::resize();
                    RenderTarget::Skip
                }
                CurrentSurfaceTexture::Validation => {
                    panic!("Validation error in get_current_texture")
                }
            },
        }
    }

    fn reconfigure_and_skip() -> RenderTarget {
        warn!("Surface is outdated, reconfiguring");
        Window::reconfigure_surface();
        RenderTarget::Skip
    }

    pub(crate) fn render(&mut self) {
        #[cfg(desktop)]
        if Window::is_resizing() {
            return;
        }

        let surface_texture = match Self::acquire_render_target() {
            RenderTarget::Skip => return,
            #[cfg(not_wasm)]
            RenderTarget::Offscreen => None,
            RenderTarget::Surface(tex) => Some(tex),
        };

        Window::next_render_frame();

        let work_started = Instant::now();

        for font in Font::storage_mut().values_mut() {
            font.brush.next_frame();
        }

        if surface_texture.is_none() {
            self.ensure_offscreen_texture();
        }
        self.ensure_depth_texture();
        self.ensure_msaa_texture();

        #[cfg(feature = "bench")]
        self.ensure_gpu_timer();

        // The surface cannot be sampled, so a frame that has to read
        // itself back renders into an intermediate scene texture and
        // is copied to the surface at the end. The headless offscreen
        // texture is sampleable as is. Frames that do not sample skip
        // all of this and render straight to the target.
        let needs_sampling = AppHandler::current().te_window_events.needs_sampleable_frame();

        // A pending read forces the scene texture path where the surface
        // cannot be copied, the readback copies the scene texture instead.
        let needs_sampling =
            needs_sampling || (!crate::window::SURFACE_COPY && self.read_display_request.borrow().is_some());

        if needs_sampling && surface_texture.is_some() {
            self.ensure_scene_texture();
        }

        let texture = match &surface_texture {
            Some(surface_texture) => &surface_texture.texture,
            None => self.offscreen_texture.as_ref().unwrap(),
        };

        let target_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let (scene_view, present_view) = if needs_sampling && surface_texture.is_some() {
            let scene = self.scene_texture.as_ref().unwrap();
            (
                scene.create_view(&wgpu::TextureViewDescriptor::default()),
                Some(target_view),
            )
        } else {
            (target_view, None)
        };

        let encoder = Window::device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        #[cfg(feature = "bench")]
        let timestamp_writes = Some(wgpu::RenderPassTimestampWrites {
            query_set:                     &self.gpu_timer.as_ref().expect("gpu timer ensured").query_set,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index:       Some(1),
        });
        #[cfg(not(feature = "bench"))]
        let timestamp_writes = None;

        let msaa_view = self
            .msaa_texture
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));

        let mut frame = RenderFrame::new(
            encoder,
            scene_view,
            present_view,
            self.depth_texture.as_ref().unwrap().view.clone(),
            msaa_view,
            self.clear_color,
            timestamp_writes,
        );

        AppHandler::current().te_window_events.render(&mut frame);

        let mut encoder = frame.finish();

        let buffer = if self.read_display_request.borrow().is_some() {
            // Where the surface cannot be copied the pending read forced
            // the sampling path above, so the scene texture holds this
            // frame. The headless offscreen texture is copyable as is.
            let source = if crate::window::SURFACE_COPY || surface_texture.is_none() {
                texture
            } else {
                self.scene_texture.as_ref().expect("Scene texture ensured by pending read")
            };

            Some(Self::read_screen(&mut encoder, source))
        } else {
            None
        };

        Window::queue().submit(std::iter::once(encoder.finish()));

        self.frame_work_time = self.update_work + work_started.elapsed().as_secs_f32();

        if let Some(surface_texture) = surface_texture {
            Window::queue().present(surface_texture);
        }

        // After the work timer closes and the frame is presented, so resolving
        // and the blocking readback never inflate the CPU measurement.
        #[cfg(feature = "bench")]
        self.read_gpu_time().expect("failed to read gpu time");

        self.deliver_pending_read(buffer);
    }

    fn deliver_pending_read(&self, buffer: Option<(wgpu::Buffer, crate::gm::flat::Size<u32>)>) {
        #[cfg(not_wasm)]
        if let Some(buffer_sender) = self.read_display_request.take() {
            let (sender, receiver) = channel();

            let Some(buffer) = buffer else {
                return;
            };

            let buffer_slice = buffer.0.slice(..);

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });

            // On demand rendering lets the loop sleep after this frame, with
            // nothing left to drive the GPU. Poll here so the map completes and
            // the screenshot is delivered no matter the frame cadence.
            if let Err(err) = Window::device().poll(wgpu::PollType::wait_indefinitely()) {
                warn!("Screenshot device poll failed: {err}");
            }

            crate::deps::hreads::spawn(async move {
                let _ = receiver.recv().unwrap();
                Self::deliver_screenshot(buffer, &buffer_sender);
            });
        }

        // The browser cannot block on the device. The map callback fires
        // on the main thread once the browser completes the copy, and the
        // waiting test thread receives through the request channel.
        #[cfg(wasm)]
        if let Some(buffer_sender) = self.read_display_request.take() {
            let Some((buffer, size)) = buffer else {
                return;
            };

            let mapped = buffer.clone();

            buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
                if let Err(err) = result {
                    warn!("Screenshot map failed: {err}");
                    return;
                }

                Self::deliver_screenshot((mapped, size), &buffer_sender);
            });
        }
    }

    #[cfg(feature = "bench")]
    fn ensure_gpu_timer(&mut self) {
        if self.gpu_timer.is_some() {
            return;
        }

        let device = Window::device();
        self.gpu_timer = Some(GpuTimer {
            query_set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("bench timestamps"),
                ty:    wgpu::QueryType::Timestamp,
                count: GPU_TIMESTAMPS,
            }),
            resolve:   device.create_buffer(&wgpu::BufferDescriptor {
                label:              Some("bench timestamp resolve"),
                size:               wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT,
                usage:              wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback:  device.create_buffer(&wgpu::BufferDescriptor {
                label:              Some("bench timestamp readback"),
                size:               GPU_TIMESTAMP_BYTES,
                usage:              wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
        });
    }

    /// Reads back the render pass timestamps resolved during `render` and
    /// blocks until the GPU reports them. The blocking poll runs after the
    /// work timer has closed, so it costs only wall-clock throughput, not
    /// `frame_work_time`.
    #[cfg(feature = "bench")]
    fn read_gpu_time(&mut self) -> Result<()> {
        let gpu_seconds = {
            let Some(timer) = self.gpu_timer.as_ref() else {
                return Ok(());
            };

            // Wait for the render submission to finish so Metal commits the
            // end-of-pass timestamp to the query set before we resolve it.
            Window::device().poll(wgpu::PollType::wait_indefinitely())?;

            let mut encoder = Window::device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bench timestamp resolve"),
            });
            encoder.resolve_query_set(&timer.query_set, 0..GPU_TIMESTAMPS, &timer.resolve, 0);
            encoder.copy_buffer_to_buffer(&timer.resolve, 0, &timer.readback, 0, GPU_TIMESTAMP_BYTES);
            Window::queue().submit(std::iter::once(encoder.finish()));

            let slice = timer.readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |result| {
                result.expect("bench timestamp readback map failed");
            });
            Window::device().poll(wgpu::PollType::wait_indefinitely())?;

            let period = Window::queue().get_timestamp_period();
            let stamps: [u64; 2] = {
                let data = slice.get_mapped_range()?;
                bytemuck::pod_read_unaligned(&data[..])
            };
            timer.readback.unmap();

            let ticks: f32 = stamps[1].saturating_sub(stamps[0]).lossy_convert();
            ticks * period / 1.0e9
        };

        self.frame_gpu_time = gpu_seconds;
        Ok(())
    }

    fn ensure_depth_texture(&mut self) {
        let size: crate::gm::flat::Size<u32> = Window::render_size().lossy_convert();

        let up_to_date = self.depth_texture.as_ref().is_some_and(|texture| texture.size == size);

        if up_to_date {
            return;
        }

        self.depth_texture = Some(Texture::create_depth_texture(
            Window::device(),
            size,
            msaa_sample_count(),
            "depth_texture",
        ));
    }

    /// The multisampled color target the frame renders into before
    /// resolving to the presentable texture. Not created when
    /// multisampling is off, then the frame renders straight into the
    /// resolve target.
    fn ensure_msaa_texture(&mut self) {
        if msaa_sample_count() == 1 {
            return;
        }

        let size = Window::render_size();
        let width: u32 = size.width.lossy_convert();
        let height: u32 = size.height.lossy_convert();

        let up_to_date = self
            .msaa_texture
            .as_ref()
            .is_some_and(|texture| texture.size().width == width && texture.size().height == height);

        if up_to_date {
            return;
        }

        self.msaa_texture = Some(Window::device().create_texture(&wgpu::TextureDescriptor {
            label:           Some("MSAA Render Texture"),
            size:            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    msaa_sample_count(),
            dimension:       wgpu::TextureDimension::D2,
            format:          surface_texture_format(),
            usage:           wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats:    &[],
        }));
    }

    fn ensure_offscreen_texture(&mut self) {
        let size = Window::render_size();
        let width: u32 = size.width.lossy_convert();
        let height: u32 = size.height.lossy_convert();

        let up_to_date = self
            .offscreen_texture
            .as_ref()
            .is_some_and(|texture| texture.size().width == width && texture.size().height == height);

        if up_to_date {
            return;
        }

        self.offscreen_texture = Some(Window::device().create_texture(&wgpu::TextureDescriptor {
            label:           Some("Offscreen Render Texture"),
            size:            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       wgpu::TextureDimension::D2,
            format:          surface_texture_format(),
            usage:           wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats:    &[],
        }));
    }

    /// Intermediate render target for frames that sample themselves,
    /// for example for backdrop blur. Windowed only, the headless
    /// offscreen texture is sampleable directly.
    fn ensure_scene_texture(&mut self) {
        let size = Window::render_size();
        let width: u32 = size.width.lossy_convert();
        let height: u32 = size.height.lossy_convert();

        let up_to_date = self
            .scene_texture
            .as_ref()
            .is_some_and(|texture| texture.size().width == width && texture.size().height == height);

        if up_to_date {
            return;
        }

        self.scene_texture = Some(Window::device().create_texture(&wgpu::TextureDescriptor {
            label:           Some("Scene Render Texture"),
            size:            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       wgpu::TextureDimension::D2,
            format:          surface_texture_format(),
            usage:           SCENE_TEXTURE_USAGE,
            view_formats:    &[],
        }));
    }

    fn deliver_screenshot(buffer: (wgpu::Buffer, crate::gm::flat::Size<u32>), sender: &ReadDisplayRequest) {
        let (buff, size) = buffer;

        if size.width == 0 || size.height == 0 {
            sender.send(Screenshot::new(vec![], size)).unwrap();
            return;
        }

        let width = usize::try_from(size.width).unwrap();
        let height = usize::try_from(size.height).unwrap();
        let real_row_bytes = width * std::mem::size_of::<crate::gm::color::U8Color>();
        let row_bytes =
            real_row_bytes.next_multiple_of(usize::try_from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT).unwrap());

        let bytes: &[u8] = &buff
            .slice(..)
            .get_mapped_range()
            .expect("buffer should be mapped before readback");

        let mut data: Vec<crate::gm::color::U8Color> = Vec::with_capacity(width * height);

        // The channel order follows the render format, bgra on desktop
        // and in mac browsers, rgba elsewhere.
        let swaps_channels = surface_texture_format() == TextureFormat::Bgra8Unorm;

        for row in bytes.chunks_exact(row_bytes) {
            let row = bytemuck::cast_slice::<u8, crate::gm::color::U8Color>(&row[..real_row_bytes]);

            if swaps_channels {
                data.extend(row.iter().map(|color| color.bgra_to_rgba()));
            } else {
                data.extend_from_slice(row);
            }
        }

        sender.send(Screenshot::new(data, size)).unwrap();
    }

    pub(crate) fn request_read_display(&self) -> Receiver<Screenshot> {
        let mut request = self.read_display_request.borrow_mut();

        let (s, r) = channel();
        request.replace(s);
        r
    }

    fn read_screen(
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
    ) -> (wgpu::Buffer, crate::gm::flat::Size<u32>) {
        let screen_width_bytes: u64 = u64::from(texture.size().width) * std::mem::size_of::<u32>() as u64;

        let width_bytes = screen_width_bytes.next_multiple_of(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT));

        let buffer = Window::device().create_buffer(&wgpu::BufferDescriptor {
            label:              Some("Read Screen Buffer"),
            size:               width_bytes * u64::from(texture.size().height),
            usage:              wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset:         0,
                    bytes_per_row:  u32::try_from(width_bytes).unwrap().into(),
                    rows_per_image: texture.size().height.into(),
                },
            },
            wgpu::Extent3d {
                width:                 texture.size().width,
                height:                texture.size().height,
                depth_or_array_layers: 1,
            },
        );

        let size: crate::gm::flat::Size<u32> =
            crate::gm::flat::Size::new(texture.size().width, texture.size().height);

        (buffer, size)
    }
}
