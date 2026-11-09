//! The rasters of an svg in a browser build, kept by the id of the svg:
//! 1 texture per size it draws at, which the page draws later, and the
//! fixed raster that sprites and levels draw.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use wgpu::BindGroup;

use crate::{
    deps::{
        hreads::{is_main_thread, on_main, spawn},
        refs::{main_lock::MainLock, manage::DataManager},
    },
    gm::flat::Size,
    image::{
        Image, ImageBind, ImageFilter, RASTER_KEEP_FRAMES, Svg, Texture, browser_decode,
        late::{Late, set_pictures_pending, transparent},
        svg_size::sized,
    },
    window::Window,
};

struct Raster {
    /// Empty while the browser draws it.
    bind:      Arc<OnceLock<ImageBind>>,
    last_used: u64,
}

#[derive(Default)]
struct SvgGpu {
    rasters: HashMap<(u32, u32), Raster>,
    /// The old fixed raster that sprites and levels draw.
    fixed:   Late,
}

static SVGS: MainLock<HashMap<u64, SvgGpu>> = MainLock::new();

impl Svg {
    fn gpu(&self) -> &'static mut SvgGpu {
        SVGS.get_mut().entry(self.id).or_default()
    }

    fn rasters(&self) -> &'static mut HashMap<(u32, u32), Raster> {
        &mut self.gpu().rasters
    }

    fn fixed(&self) -> &'static mut Late {
        &mut self.gpu().fixed
    }

    /// Marks the raster of `size` as used on `frame`, asking the browser
    /// for it first when this size was never drawn or was dropped.
    pub(crate) fn touch(&self, size: Size<u32>, frame: u64) {
        let size = Size::new(size.width.max(1), size.height.max(1));
        let rasters = self.rasters();
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
            .rasters()
            .values()
            .filter(|raster| raster.last_used >= frame && raster.bind.get().is_none())
            .count();
        rasters + usize::from(self.fixed().is_pending())
    }

    /// Whether the raster of `size` is drawn, so the damage of a view
    /// changes when it lands.
    pub(crate) fn is_ready(&self, size: Size<u32>) -> bool {
        let size = Size::new(size.width.max(1), size.height.max(1));
        self.rasters()
            .get(&(size.width, size.height))
            .is_some_and(|raster| raster.bind.get().is_some())
    }

    /// Runs `draw` with the bind group of `size`, a transparent one while
    /// the browser draws it.
    pub(crate) fn with_bind(&self, size: Size<u32>, draw: impl FnOnce(&BindGroup)) {
        let size = Size::new(size.width.max(1), size.height.max(1));
        match self
            .rasters()
            .get(&(size.width, size.height))
            .and_then(|raster| raster.bind.get())
        {
            Some(bind) => draw(&bind.bind),
            None => draw(&transparent().bind),
        }
    }

    /// The fixed raster that sprites and levels draw, asked of the browser
    /// on the first call and transparent until it lands.
    pub(crate) fn fixed_bind(&self) -> &BindGroup {
        if let Some(bind) = self.fixed().get() {
            return &bind.bind;
        }
        let size = self.fixed_size;
        let data = sized(&self.text, self.size, size.width, size.height);
        self.fixed().start("the fixed raster of an svg".to_string(), async move {
            browser_decode::rasterize_svg(data.as_bytes(), size).await
        });
        &transparent().bind
    }

    /// The filter of the fixed raster, like `Image::set_filter` sets it on
    /// the other platforms. The rasters of a view keep linear.
    pub(crate) fn set_filter(&mut self, filter: ImageFilter) {
        self.fixed().set_filter(filter);
    }

    pub(crate) fn drop_stale(&self, frame: u64) {
        self.rasters()
            .retain(|_, raster| raster.last_used + RASTER_KEEP_FRAMES >= frame);
    }

    /// Every cached raster size in pixels, the ones the browser still draws
    /// too, in no particular order.
    pub fn raster_sizes(&self) -> Vec<Size<u32>> {
        self.rasters()
            .keys()
            .map(|(width, height)| Size::new(*width, *height))
            .collect()
    }

    /// Runs at the end of every frame. Also counts the pictures this frame
    /// drew that the browser still makes, see `late::pictures_pending`.
    pub(crate) fn drop_stale_everywhere() {
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

impl Drop for Svg {
    fn drop(&mut self) {
        let id = self.id;
        if is_main_thread() {
            SVGS.get_mut().remove(&id);
        } else {
            on_main(move || {
                SVGS.get_mut().remove(&id);
            });
        }
    }
}
