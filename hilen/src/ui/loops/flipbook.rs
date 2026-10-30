//! The pictures of a loop, drawn by the pipelines of the engine. The pose
//! sets the view to each phase, and the view and its subviews are drawn
//! alone into a texture the size of the picture. One submit per picture,
//! the uploads of a picture run at its submit and the next picture writes
//! the same buffers again. Nothing waits for the GPU: the pictures are
//! copied into 1 buffer, and the frames after this one ask whether the GPU
//! is done with it.

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
    deps::refs::manage::DataManager,
    gm::{
        color::CLEAR,
        flat::{Rect, Size},
    },
    monotonic::monotonic_seconds,
    render::upload_pending,
    ui::{UIDrawer, WeakView},
    window::{Font, RenderFrame, Window, image::Texture, msaa_sample_count, surface_texture_format},
};

/// Most bytes of the pictures of 1 loop. A loop that needs more stays on
/// the frame path.
const MAX_BYTES: u64 = 64 << 20;

/// How many sets of pictures were drawn since the start.
#[cfg(feature = "ui-tests")]
static DRAWS: AtomicU32 = AtomicU32::new(0);

/// How many sets of pictures were drawn since the start.
#[cfg(feature = "ui-tests")]
pub(super) fn picture_draws() -> u32 {
    DRAWS.load(Ordering::Relaxed)
}

/// Pictures the GPU draws now.
pub(super) struct Drawing {
    buffer:  wgpu::Buffer,
    mapped:  Receiver<Result<(), BufferAsyncError>>,
    size:    Size<u32>,
    /// The bytes of a row in the buffer, a whole number of the copy steps.
    row:     u64,
    /// When the draw went out, in the seconds of `monotonic_seconds`.
    started: f64,
}

/// The view posed at each of `phases` and drawn alone, cut at `pixels`.
/// It goes out to the GPU and does not wait for it.
pub(super) fn draw(view: WeakView, pixels: Rect<u32>, phases: &[f32], pose: &mut Pose) -> Result<Drawing> {
    ensure!(
        surface_texture_format() == TextureFormat::Bgra8Unorm,
        "a frame of {:?} has no picture format",
        surface_texture_format()
    );

    let started = monotonic_seconds();
    let width = pixels.width();
    let height = pixels.height();
    let row = u64::from(width * 4).next_multiple_of(u64::from(COPY_BYTES_PER_ROW_ALIGNMENT));
    let one = row * u64::from(height);
    let count = u64::try_from(phases.len())?;
    ensure!(
        one * count <= MAX_BYTES,
        "{count} pictures of {width} by {height} pixels are more than {MAX_BYTES} bytes"
    );

    let size = Size::new(width, height);
    let targets = Targets::new(size);
    let device = Window::device();

    let buffer = device.create_buffer(&BufferDescriptor {
        label:              Some("Loop pictures"),
        size:               one * count,
        usage:              BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // The frame path skips a view on a layer, these pictures are of it.
    let base = view.__base_view();
    let on_layer = base.loop_layer;
    base.loop_layer = false;

    for (index, phase) in phases.iter().enumerate() {
        pose(*phase);
        UIDrawer::update_subtree(view);

        for font in Font::storage_mut().values_mut() {
            font.begin_frame();
        }

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

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture:   &targets.color,
                mip_level: 0,
                origin:    Origin3d::ZERO,
                aspect:    TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
                    offset:         one * u64::try_from(index)?,
                    bytes_per_row:  Some(u32::try_from(row)?),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        upload_pending();
        Window::queue().submit(std::iter::once(encoder.finish()));
    }

    view.__base_view().loop_layer = on_layer;

    // The map is done once the GPU ran every copy above.
    let (sender, mapped) = channel();
    buffer.slice(..).map_async(MapMode::Read, move |result| {
        if sender.send(result).is_err() {
            debug!("the pictures of a loop were mapped after their loop was gone");
        }
    });
    #[cfg(feature = "ui-tests")]
    DRAWS.fetch_add(1, Ordering::Relaxed);
    debug!(
        "{count} pictures of {width} by {height} pixels went to the GPU in {:.1} ms",
        (monotonic_seconds() - started) * 1000.0
    );

    Ok(Drawing {
        buffer,
        mapped,
        size,
        row,
        started,
    })
}

impl Drawing {
    /// The pictures once the GPU is done with them, none before. It never
    /// waits.
    pub(super) fn poll(&self) -> Option<Result<RawFrames>> {
        let result = match self.mapped.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => {
                // The map callback runs in a poll of the device.
                if let Err(error) = Window::device().poll(PollType::Poll) {
                    return Some(Err(anyhow!("the device was lost: {error}")));
                }
                match self.mapped.try_recv() {
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
