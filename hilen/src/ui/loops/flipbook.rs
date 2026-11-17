//! The pictures of a loop, drawn by the pipelines of the engine. The pose
//! sets the view to each phase, and the view and its subviews are drawn
//! alone into a texture the size of the picture. One submit per picture,
//! the uploads of a picture run at its submit and the next picture writes
//! the same buffers again. The draw is spread over the steps of the loops,
//! each step draws pictures until `STEP_BUDGET` is used. Nothing waits for
//! the GPU: the pictures are copied into 1 buffer, and the frames after the
//! last picture ask whether the GPU is done with it.

#[cfg(feature = "ui-tests")]
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    ops::Deref,
    sync::mpsc::{Receiver, TryRecvError, channel},
};

use anyhow::{Result, anyhow, bail, ensure};
use log::debug;
use wgpu::{
    BufferAsyncError, BufferDescriptor, BufferUsages, COPY_BYTES_PER_ROW_ALIGNMENT, CommandEncoderDescriptor,
    Extent3d, MapMode, Origin3d, PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};

use super::{
    Pose,
    on_layer::{PixelOrder, RawFrames},
};
use crate::{
    deps::refs::main_lock::MainLock,
    gm::{
        LossyConvert,
        color::CLEAR,
        flat::{Rect, Size},
    },
    image::Texture,
    monotonic::monotonic_seconds,
    render::upload_pending,
    text::TextBrushes,
    ui::{UIDrawer, WeakView},
    window::{RenderFrame, Window, msaa_sample_count, surface_texture_format},
};

/// Most bytes of the pictures of 1 loop. A loop that needs more stays on
/// the frame path.
const MAX_BYTES: u64 = 64 << 20;

/// How long the pictures may take of 1 step of the loops, in seconds. All
/// 145 pictures of a mark in 1 step took 26 ms on a Mac and 86 ms on a
/// GitHub mac, more than 1 frame.
const STEP_BUDGET: f64 = 0.008;

/// The count of the step and when its budget for pictures ends.
#[derive(Default)]
struct Step {
    count: u64,
    ends:  f64,
}

static STEP: MainLock<Step> = MainLock::new();

/// A new step of the loops starts, with a new budget for pictures.
pub(super) fn begin_step() {
    let step = STEP.get_mut();
    step.count += 1;
    step.ends = monotonic_seconds() + STEP_BUDGET;
}

/// How many sets of pictures were drawn since the start.
#[cfg(feature = "ui-tests")]
static DRAWS: AtomicU32 = AtomicU32::new(0);

/// How many sets of pictures were drawn since the start.
#[cfg(feature = "ui-tests")]
pub(super) fn picture_draws() -> u32 {
    DRAWS.load(Ordering::Relaxed)
}

/// Pictures that are drawn now, by the steps and then by the GPU.
pub(super) struct Drawing {
    buffer:  wgpu::Buffer,
    /// `None` until every picture went out.
    mapped:  Option<Receiver<Result<(), BufferAsyncError>>>,
    size:    Size<u32>,
    /// The bytes of a row in the buffer, a whole number of the copy steps.
    row:     u64,
    /// When the draw started, in the seconds of `monotonic_seconds`.
    started: f64,
    phases:  Vec<f32>,
    /// The next picture to draw.
    next:    usize,
    /// The steps that drew pictures.
    steps:   u32,
    /// The step that drew pictures last.
    step:    u64,
    /// Dropped once every picture went out.
    targets: Option<Targets>,
}

/// A set of pictures of `size`, at each of `phases`. It draws nothing yet,
/// `Drawing::draw` does in the steps that follow.
pub(super) fn start(size: Size<u32>, phases: &[f32]) -> Result<Drawing> {
    ensure!(
        surface_texture_format() == TextureFormat::Bgra8Unorm,
        "a frame of {:?} has no picture format",
        surface_texture_format()
    );
    ensure!(!phases.is_empty(), "a loop with no phase has no pictures");

    let row = u64::from(size.width * 4).next_multiple_of(u64::from(COPY_BYTES_PER_ROW_ALIGNMENT));
    let one = row * u64::from(size.height);
    let count = u64::try_from(phases.len())?;
    ensure!(
        one * count <= MAX_BYTES,
        "{count} pictures of {} by {} pixels are more than {MAX_BYTES} bytes",
        size.width,
        size.height
    );

    let buffer = Window::device().create_buffer(&BufferDescriptor {
        label:              Some("Loop pictures"),
        size:               one * count,
        usage:              BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    #[cfg(feature = "ui-tests")]
    DRAWS.fetch_add(1, Ordering::Relaxed);

    Ok(Drawing {
        buffer,
        mapped: None,
        size,
        row,
        started: monotonic_seconds(),
        phases: phases.to_vec(),
        next: 0,
        steps: 0,
        step: 0,
        targets: Some(Targets::new(size)),
    })
}

impl Drawing {
    /// Draws the next pictures with `view` posed at their phases, cut at
    /// `pixels`, until the budget of this step is used, and at least 1 in
    /// a step. Every loop that holds the set has the same content, so any
    /// of them can go on with it, each with the pixels of its own view.
    /// The last picture maps the buffer, with no wait for the GPU.
    pub(super) fn draw(&mut self, view: WeakView, pixels: Rect<u32>, pose: &mut Pose) -> Result<()> {
        if self.mapped.is_some() {
            return Ok(());
        }
        ensure!(
            pixels.size == self.size,
            "a loop of {:?} pixels cannot go on with pictures of {:?}",
            pixels.size,
            self.size
        );

        // The frame path skips a view on a layer, these pictures are of it.
        let base = view.__base_view();
        let on_layer = base.loop_layer;
        base.loop_layer = false;
        let drawn = self.draw_in_budget(view, pixels, pose);
        view.__base_view().loop_layer = on_layer;
        drawn?;

        if self.next < self.phases.len() {
            return Ok(());
        }
        self.targets = None;
        // The map is done once the GPU ran every copy above.
        let (sender, mapped) = channel();
        self.buffer.slice(..).map_async(MapMode::Read, move |result| {
            if sender.send(result).is_err() {
                debug!("the pictures of a loop were mapped after their loop was gone");
            }
        });
        self.mapped = Some(mapped);
        debug!(
            "{} pictures of {} by {} pixels went to the GPU in {} steps, {:.1} ms after the start",
            self.phases.len(),
            self.size.width,
            self.size.height,
            self.steps,
            (monotonic_seconds() - self.started) * 1000.0
        );
        Ok(())
    }

    fn draw_in_budget(&mut self, view: WeakView, pixels: Rect<u32>, pose: &mut Pose) -> Result<()> {
        let step = STEP.get_mut();
        if self.step != step.count {
            self.step = step.count;
            self.steps += 1;
            self.draw_one(view, pixels, pose)?;
        }
        while self.next < self.phases.len() && monotonic_seconds() < step.ends {
            self.draw_one(view, pixels, pose)?;
        }
        Ok(())
    }

    fn draw_one(&mut self, view: WeakView, pixels: Rect<u32>, pose: &mut Pose) -> Result<()> {
        let Some(targets) = &self.targets else {
            bail!("the pictures of a loop have no targets left");
        };
        let Some(phase) = self.phases.get(self.next).copied() else {
            return Ok(());
        };
        pose(phase);
        UIDrawer::update_subtree(view);

        let size: Size<u32> = Window::render_size().lossy_convert();
        TextBrushes::begin_frame(size);

        let device = Window::device();
        let encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Loop picture"),
        });
        let mut frame = RenderFrame::new(
            encoder,
            targets.color.create_view(&TextureViewDescriptor::default()),
            None,
            targets.depth.view.clone(),
            targets
                .msaa
                .as_ref()
                .map(|msaa| msaa.create_view(&TextureViewDescriptor::default())),
            Some(CLEAR),
            None,
        );
        UIDrawer::draw_subtree(&mut frame, view.deref(), pixels);
        let mut encoder = frame.finish();

        let one = self.row * u64::from(self.size.height);
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture:   &targets.color,
                mip_level: 0,
                origin:    Origin3d::ZERO,
                aspect:    TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: TexelCopyBufferLayout {
                    offset:         one * u64::try_from(self.next)?,
                    bytes_per_row:  Some(u32::try_from(self.row)?),
                    rows_per_image: Some(self.size.height),
                },
            },
            Extent3d {
                width:                 self.size.width,
                height:                self.size.height,
                depth_or_array_layers: 1,
            },
        );

        upload_pending();
        Window::queue().submit(std::iter::once(encoder.finish()));
        self.next += 1;
        Ok(())
    }

    /// The pictures once the GPU is done with them, none before. It never
    /// waits.
    pub(super) fn poll(&self) -> Option<Result<RawFrames>> {
        let mapped = self.mapped.as_ref()?;
        let result = match mapped.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => {
                // The map callback runs in a poll of the device.
                if let Err(error) = Window::device().poll(PollType::Poll) {
                    return Some(Err(anyhow!("the device was lost: {error}")));
                }
                match mapped.try_recv() {
                    Ok(result) => result,
                    Err(TryRecvError::Empty) => return None,
                    Err(TryRecvError::Disconnected) => {
                        return Some(Err(anyhow!("the pictures of a loop were never mapped")));
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                return Some(Err(anyhow!("the pictures of a loop were never mapped")));
            }
        };
        Some(self.read(result))
    }

    fn read(&self, mapped: Result<(), BufferAsyncError>) -> Result<RawFrames> {
        if let Err(error) = mapped {
            bail!("the pictures of a loop could not be mapped: {error}");
        }
        let width = self.size.width;
        let tight = usize::try_from(width * 4)?;
        let row = usize::try_from(self.row)?;
        let rows = usize::try_from(self.size.height)?;
        let mapped = self.buffer.slice(..).get_mapped_range()?;
        let pixels = mapped
            .chunks_exact(row * rows)
            .map(|picture| {
                let mut pixels = Vec::with_capacity(tight * rows);
                for line in picture.chunks_exact(row) {
                    pixels.extend_from_slice(&line[..tight]);
                }
                pixels
            })
            .collect();
        drop(mapped);
        self.buffer.unmap();
        debug!(
            "the pictures of {width} by {} pixels were back from the GPU after {:.1} ms",
            self.size.height,
            (monotonic_seconds() - self.started) * 1000.0
        );
        Ok(RawFrames {
            pixels,
            size: self.size,
            order: PixelOrder::BgraPremultiplied,
        })
    }
}

/// What the pictures draw into, made for each set of pictures and dropped
/// after it. A color of the picture size with the depth and the samples a
/// frame has.
struct Targets {
    color: wgpu::Texture,
    msaa:  Option<wgpu::Texture>,
    depth: Texture,
}

impl Targets {
    fn new(size: Size<u32>) -> Self {
        let device = Window::device();
        let samples = msaa_sample_count();
        let color = |label: &str, sample_count: u32, usage: TextureUsages| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width:                 size.width,
                    height:                size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: TextureDimension::D2,
                format: surface_texture_format(),
                usage,
                view_formats: &[],
            })
        };

        Self {
            color: color(
                "Loop picture",
                1,
                TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            ),
            msaa:  (samples > 1)
                .then(|| color("Loop picture samples", samples, TextureUsages::RENDER_ATTACHMENT)),
            depth: Texture::create_depth_texture(device, size, samples, "Loop picture depth"),
        }
    }
}
