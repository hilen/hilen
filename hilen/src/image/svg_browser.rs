//! An svg in a browser build. The page draws it, so the wasm carries no svg
//! code: `usvg`, `resvg` and their parsers. An LG C1 compiles about 2.3 s of
//! start per MB of code. The browser draws each pixel size the first time a
//! frame asks for it, see `browser_decode`, so a size shows a frame or more
//! after its first draw, transparent until then.

use std::{str::from_utf8, sync::Arc};

use anyhow::{Result, anyhow};

use crate::{
    gm::flat::Size,
    image::{
        next_gpu_id,
        svg_size::{fixed_side, intrinsic_size},
    },
};

/// Frames a raster stays cached after the last frame that drew it. A
/// resize animation that goes and comes back inside this window reuses
/// its rasters instead of building them again.
pub const RASTER_KEEP_FRAMES: u64 = 60;

#[derive(Debug)]
pub struct Svg {
    pub(super) text:       Arc<str>,
    /// The width and height the svg draws at, before any scale.
    pub(super) size:       (f32, f32),
    pub(super) fixed_size: Size<u32>,
    /// The drawer keeps the rasters of the svg by this id, see
    /// `svg_browser_gpu.rs`.
    pub(super) id:         u64,
}

impl Svg {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self> {
        let text = from_utf8(bytes)?;
        let (width, height) =
            intrinsic_size(text).ok_or_else(|| anyhow!("An svg with no <svg> tag or no size"))?;
        Ok(Self {
            text:       text.into(),
            size:       (width, height),
            fixed_size: Size::new(fixed_side(width), fixed_side(height)),
            id:         next_gpu_id(),
        })
    }

    pub(crate) fn is_svg(bytes: &[u8]) -> bool {
        bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml")
    }

    /// The size of the fixed raster, which is also the size of its image,
    /// eight times the svg like on every other platform.
    pub(crate) fn fixed_size(&self) -> Size<u32> {
        self.fixed_size
    }
}
