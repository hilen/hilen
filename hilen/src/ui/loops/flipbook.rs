//! The pictures of a loop, drawn by the pipelines of the engine. The pose
//! sets the view to each phase, the view and its subviews are drawn alone
//! into a texture the size of the window, and the pixels of the view are
//! copied out. One submit per picture, the uploads of a picture run at its
//! submit and the next picture writes the same buffers again.

use std::{ops::Deref, sync::mpsc::channel};

use anyhow::{Context, Result, anyhow, ensure};
use wgpu::{
    BufferDescriptor, BufferUsages, COPY_BYTES_PER_ROW_ALIGNMENT, CommandEncoderDescriptor, Extent3d,
    MapMode, Origin3d, PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};

use super::{
    Pose,
    on_layer::{PixelOrder, RawFrames},
};
use crate::{
    deps::refs::manage::DataManager,
    gm::{
        LossyConvert,
        color::CLEAR,
        flat::{Rect, Size},
    },
    render::upload_pending,
    ui::{UIDrawer, UIManager, WeakView},
    window::{Font, RenderFrame, Window, image::Texture, msaa_sample_count, surface_texture_format},
};

/// Most bytes of the pictures of 1 loop. A loop that needs more stays on
/// the frame path.
const MAX_BYTES: u64 = 64 << 20;

/// The view posed at each of `phases` and drawn alone, cut at `pixels`.
pub(super) fn draw(view: WeakView, pixels: Rect<u32>, phases: &[f32], pose: &mut Pose) -> Result<RawFrames> {
    ensure!(
        surface_texture_format() == TextureFormat::Bgra8Unorm,
        "a frame of {:?} has no picture format",
        surface_texture_format()
    );

    let width = pixels.width();
    let height = pixels.height();
    let row = u64::from(width * 4).next_multiple_of(u64::from(COPY_BYTES_PER_ROW_ALIGNMENT));
    let one = row * u64::from(height);
    let count = u64::try_from(phases.len())?;
    ensure!(
        one * count <= MAX_BYTES,
        "{count} pictures of {width} by {height} pixels are more than {MAX_BYTES} bytes"
    );

    let window: Size<u32> = UIManager::window_resolution().lossy_convert();
    let targets = Targets::new(window);
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
                origin:    Origin3d {
                    x: pixels.x(),
                    y: pixels.y(),
                    z: 0,
                },
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

    Ok(RawFrames {
        pixels: read_back(&buffer, width, height, row)?,
        size:   Size::new(width, height),
        order:  PixelOrder::BgraPremultiplied,
    })
}

/// The pictures out of `buffer`, each `height` rows of `row` bytes, cut to
/// `width` pixels a row.
fn read_back(buffer: &wgpu::Buffer, width: u32, height: u32, row: u64) -> Result<Vec<Vec<u8>>> {
    let device = Window::device();
    let (sender, receiver) = channel();
    buffer.slice(..).map_async(MapMode::Read, move |result| {
        if sender.send(result).is_err() {
            log::warn!("the pictures of a loop were mapped after the wait for them ended");
        }
    });
    device.poll(PollType::wait_indefinitely())?;
    receiver
        .recv()
        .context("the pictures of a loop were never mapped")?
        .map_err(|error| anyhow!("the pictures of a loop could not be mapped: {error}"))?;

    let tight = usize::try_from(width * 4)?;
    let row = usize::try_from(row)?;
    let rows = usize::try_from(height)?;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let frames = mapped
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
    buffer.unmap();
    Ok(frames)
}

/// What the pictures draw into, made for each set of pictures and dropped
/// after it. A window size color with the depth and the samples a frame has.
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
