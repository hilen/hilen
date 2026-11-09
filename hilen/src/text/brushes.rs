//! The GPU brush of each font, kept here by the id of the font so that
//! `Font` holds only what measures and shapes text. A brush is made the
//! first time its font draws.

use std::{collections::HashMap, mem::take};

use anyhow::Result;
use wgpu::{MultisampleState, RenderPass};
use wgpu_text::{BrushBuilder, Section, TextBrush};

use crate::{
    deps::refs::main_lock::MainLock,
    gm::{LossyConvert, flat::Size},
    render::depth_stencil_state,
    text::{Font, ShapedLayout, ShapedParams},
    window::{Window, msaa_sample_count, surface_texture_format},
};

struct FontBrush {
    brush:       TextBrush,
    /// No batch of this frame reached the brush yet. The first batch of a
    /// frame starts the brush's vertex buffer over and every later one
    /// appends, so the earlier draws of the frame keep their vertices.
    first_batch: bool,
    /// The size in pixels the view matrix of the brush is set to. Each
    /// window draws with its own size, see `begin_frame`.
    view_size:   Size<u32>,
}

static BRUSHES: MainLock<HashMap<u64, FontBrush>> = MainLock::new();
/// The size of the frame being drawn, for a brush made inside it.
static FRAME_SIZE: MainLock<Option<Size<u32>>> = MainLock::new();

pub(crate) struct TextBrushes;

impl TextBrushes {
    /// Queues a section laid out with the base font of `params` and drawn
    /// with `font`, which keeps only its own glyphs. Call `process` once
    /// per batch after all its sections.
    pub(crate) fn queue(font: &Font, section: Section, params: ShapedParams) {
        let layout = ShapedLayout {
            emit: &font.name,
            params,
        };
        Self::brush(font).brush.queue_custom_layout(section, &layout);
    }

    pub(crate) fn process(font: &Font) -> Result<()> {
        font.sweep_caches();
        let device = &Window::current().device;
        let brush = Self::brush(font);
        if take(&mut brush.first_batch) {
            brush.brush.process_queued(device, Window::queue())?;
        } else {
            brush.brush.process_queued_append(device, Window::queue())?;
        }
        Ok(())
    }

    pub(crate) fn draw(font: &Font, pass: &mut RenderPass) {
        Self::brush(font).brush.draw(pass);
    }

    /// A frame of a window of `size` pixels starts. The view matrix of each
    /// brush follows the window, an extra window can have another size than
    /// the main one and they draw one after the other.
    pub(crate) fn begin_frame(size: Size<u32>) {
        *FRAME_SIZE.get_mut() = Some(size);
        for brush in BRUSHES.get_mut().values_mut() {
            brush.first_batch = true;
            if brush.view_size != size {
                brush.view_size = size;
                brush.brush.resize_view(
                    size.width.lossy_convert(),
                    size.height.lossy_convert(),
                    Window::queue(),
                );
            }
        }
    }

    /// Drops every brush, for a new sample count. A brush is a pipeline,
    /// and a pipeline is built for 1 count. Each font makes its brush again
    /// at its next draw, and its glyph pictures are drawn again then.
    pub(crate) fn rebuild() {
        BRUSHES.get_mut().clear();
    }

    fn brush(font: &Font) -> &'static mut FontBrush {
        BRUSHES.get_mut().entry(font.id()).or_insert_with(|| {
            let size = FRAME_SIZE.get_mut().unwrap_or_else(|| Window::render_size().lossy_convert());
            FontBrush {
                brush:       make_brush(font, size),
                first_batch: true,
                view_size:   size,
            }
        })
    }
}

/// The brush that draws the glyphs of `font`, built for the sample count
/// as it is now and a view of `size` pixels.
fn make_brush(font: &Font, size: Size<u32>) -> TextBrush {
    BrushBuilder::using_font(font.ab().clone())
        .with_depth_stencil(depth_stencil_state().into())
        .with_multisample(MultisampleState {
            count:                     msaa_sample_count(),
            mask:                      !0,
            alpha_to_coverage_enabled: false,
        })
        .with_stem_darkening(font.stem_darkening())
        // The glyph cache draws on the global rayon pool, which never
        // ends. A hot build that is stopped must leave no thread.
        .with_multithread(!cfg!(hot))
        .build(
            &Window::current().device,
            size.width,
            size.height,
            surface_texture_format(),
        )
}
