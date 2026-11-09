//! The rasters of an svg on the GPU, 1 texture per size it draws at, kept
//! by the id of the svg. Sizes not drawn for `RASTER_KEEP_FRAMES` frames
//! are dropped.

use std::collections::HashMap;

use crate::{
    deps::{
        hreads::{is_main_thread, on_main},
        refs::{main_lock::MainLock, manage::DataManager},
    },
    gm::flat::Size,
    image::{Image, ImageBind, RASTER_KEEP_FRAMES, Svg, Texture, svg::rasterize},
    window::Window,
};

struct Raster {
    bind:      ImageBind,
    last_used: u64,
}

/// The rasters of 1 svg by width and height.
type Rasters = HashMap<(u32, u32), Raster>;

/// By svg id.
static RASTERS: MainLock<HashMap<u64, Rasters>> = MainLock::new();

impl Svg {
    fn rasters(&self) -> &'static mut Rasters {
        RASTERS.get_mut().entry(self.id).or_default()
    }

    /// Marks the raster of `size` as used on `frame`, building it first
    /// when this size was never drawn or was dropped.
    pub(crate) fn touch(&self, size: Size<u32>, frame: u64) {
        let raster = self.rasters().entry((size.width, size.height)).or_insert_with(|| {
            let texture = Texture::from_raw_data(rasterize(&self.tree, size), "svg_raster");
            Raster {
                bind:      Image::bind_texture(&texture),
                last_used: frame,
            }
        });
        raster.last_used = frame;
    }

    /// Runs `draw` with the bind group of `size`. The raster must have
    /// been touched this frame, so it exists.
    pub(crate) fn with_bind(&self, size: Size<u32>, draw: impl FnOnce(&wgpu::BindGroup)) {
        let raster = self
            .rasters()
            .get(&(size.width, size.height))
            .expect("svg raster drawn without being touched this frame");
        draw(&raster.bind.bind);
    }

    /// Always true here, a raster is built in the frame that first draws
    /// it. A browser draws it later, see `svg_browser.rs`.
    #[allow(clippy::unused_self)]
    pub(crate) fn is_ready(&self, _size: Size<u32>) -> bool {
        true
    }

    pub(crate) fn drop_stale(&self, frame: u64) {
        self.rasters()
            .retain(|_, raster| raster.last_used + RASTER_KEEP_FRAMES >= frame);
    }

    /// Every cached raster size in pixels, in no particular order.
    pub fn raster_sizes(&self) -> Vec<Size<u32>> {
        self.rasters()
            .keys()
            .map(|(width, height)| Size::new(*width, *height))
            .collect()
    }

    pub(crate) fn drop_stale_everywhere() {
        let frame = Window::render_frame();
        for image in Image::storage().values() {
            if let Some(svg) = &image.svg {
                svg.drop_stale(frame);
            }
        }
    }
}

impl Drop for Svg {
    fn drop(&mut self) {
        let id = self.id;
        if is_main_thread() {
            RASTERS.get_mut().remove(&id);
        } else {
            on_main(move || {
                RASTERS.get_mut().remove(&id);
            });
        }
    }
}
