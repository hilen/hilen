//! An svg in a browser build. The page draws it, so the wasm carries no svg
//! code: `usvg`, `resvg` and their parsers. An LG C1 compiles about 2.3 s of
//! start per MB of code. The browser draws each pixel size the first time a
//! frame asks for it, see `browser_decode`, so a size shows a frame or more
//! after its first draw, transparent until then.

use std::{
    collections::HashMap,
    str::from_utf8,
    sync::{Arc, OnceLock},
};

use anyhow::{Result, anyhow};
use parking_lot::Mutex;
use wgpu::BindGroup;

use crate::{
    deps::hreads::spawn,
    gm::flat::Size,
    window::{
        Window,
        image::{
            Image, ImageBind, ImageFilter, Texture, browser_decode,
            late::{Late, set_pictures_pending, transparent},
            svg_size::{fixed_side, intrinsic_size, sized},
        },
    },
};

/// Frames a raster stays cached after the last frame that drew it. A
/// resize animation that goes and comes back inside this window reuses
/// its rasters instead of building them again.
pub const RASTER_KEEP_FRAMES: u64 = 60;

#[derive(Debug)]
struct Raster {
    /// Empty while the browser draws it.
    bind:      Arc<OnceLock<ImageBind>>,
    last_used: u64,
}

/// The svg text kept next to its image so an `ImageView` can have it drawn
/// at the exact pixel size it draws, one texture per size. Sizes not drawn
/// for `RASTER_KEEP_FRAMES` are dropped.
#[derive(Debug)]
pub struct Svg {
    text:       Arc<str>,
    /// The width and height the svg draws at, before any scale.
    size:       (f32, f32),
    fixed_size: Size<u32>,
    rasters:    Mutex<HashMap<(u32, u32), Raster>>,
    fixed:      Late,
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
            rasters:    Mutex::default(),
            fixed:      Late::default(),
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

    /// Marks the raster of `size` as used on `frame`, asking the browser
    /// for it first when this size was never drawn or was dropped.
    pub(crate) fn touch(&self, size: Size<u32>, frame: u64) {
        let size = Size::new(size.width.max(1), size.height.max(1));
        let mut rasters = self.rasters.lock();
        if let Some(raster) = rasters.get_mut(&(size.width, size.height)) {
            raster.last_used = frame;
            return;
        }
        let bind = Arc::new(OnceLock::new());
        let landing = Arc::downgrade(&bind);
        rasters.insert(
            (size.width, size.height),
            Raster {
                bind,
                last_used: frame,
            },
        );
        drop(rasters);

        let data = sized(&self.text, self.size, size.width, size.height);
        spawn(async move {
            match browser_decode::rasterize_svg(data.as_bytes(), size).await {
                // Gone when the size was dropped while it was drawn.
                Ok(raw) => {
                    if let Some(landing) = landing.upgrade() {
                        let texture = Texture::from_raw_data(raw, "svg_raster");
                        let _ = landing.set(Image::bind_texture(&texture));
                    }
                }
                Err(err) => log::error!("The browser did not draw an svg at {size:?}: {err}"),
            }
        });
    }

    /// Rasters drawn on `frame` or later that the browser still makes, and
    /// the fixed raster while it is made.
    pub(crate) fn pending_since(&self, frame: u64) -> usize {
        let rasters = self
            .rasters
            .lock()
            .values()
            .filter(|raster| raster.last_used >= frame && raster.bind.get().is_none())
            .count();
        rasters + usize::from(self.fixed.is_pending())
    }

    /// Whether the raster of `size` is drawn, so the damage of a view
    /// changes when it lands.
    pub(crate) fn is_ready(&self, size: Size<u32>) -> bool {
        let size = Size::new(size.width.max(1), size.height.max(1));
        self.rasters
            .lock()
            .get(&(size.width, size.height))
            .is_some_and(|raster| raster.bind.get().is_some())
    }

    /// Runs `draw` with the bind group of `size`, a transparent one while
    /// the browser draws it.
    pub(crate) fn with_bind(&self, size: Size<u32>, draw: impl FnOnce(&BindGroup)) {
        let size = Size::new(size.width.max(1), size.height.max(1));
        let rasters = self.rasters.lock();
        match rasters.get(&(size.width, size.height)).and_then(|raster| raster.bind.get()) {
            Some(bind) => draw(&bind.bind),
            None => draw(&transparent().bind),
        }
    }

    /// The fixed raster that sprites and levels draw, asked of the browser
    /// on the first call and transparent until it lands.
    pub(crate) fn fixed_bind(&self) -> &BindGroup {
        if let Some(bind) = self.fixed.get() {
            return &bind.bind;
        }
        let size = self.fixed_size;
        let data = sized(&self.text, self.size, size.width, size.height);
        self.fixed.start("the fixed raster of an svg".to_string(), async move {
            browser_decode::rasterize_svg(data.as_bytes(), size).await
        });
        &transparent().bind
    }

    /// The filter of the fixed raster, like `Image::set_filter` sets it on
    /// the other platforms. The rasters of a view keep linear.
    pub(crate) fn set_filter(&mut self, filter: ImageFilter) {
        self.fixed.set_filter(filter);
    }

    pub(crate) fn drop_stale(&self, frame: u64) {
        self.rasters
            .lock()
            .retain(|_, raster| raster.last_used + RASTER_KEEP_FRAMES >= frame);
    }

    /// Every cached raster size in pixels, the ones the browser still draws
    /// too, in no particular order.
    pub fn raster_sizes(&self) -> Vec<Size<u32>> {
        self.rasters
            .lock()
            .keys()
            .map(|(width, height)| Size::new(*width, *height))
            .collect()
    }

    /// Runs at the end of every frame. Also counts the pictures this frame
    /// drew that the browser still makes, see `late::pictures_pending`.
    pub(crate) fn drop_stale_everywhere() {
        use crate::deps::refs::manage::DataManager;

        let frame = Window::render_frame();
        let mut pending = 0;
        for image in Image::storage().values() {
            if let Some(svg) = &image.svg {
                svg.drop_stale(frame);
            }
            pending += image.pending_since(frame);
        }
        set_pictures_pending(pending);
    }
}
