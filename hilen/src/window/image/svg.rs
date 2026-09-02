use std::collections::HashMap;

use anyhow::Result;
use parking_lot::Mutex;
use resvg::{
    render,
    tiny_skia::Pixmap,
    usvg::{ImageRendering, Options, ShapeRendering, TextRendering, Transform, Tree},
};

use crate::{
    gm::{LossyConvert, flat::Size},
    window::{
        Window,
        image::{Image, ImageBind, Texture, TextureRawData},
    },
};

/// Frames a raster stays cached after the last frame that drew it. A
/// resize animation that goes and comes back inside this window reuses
/// its rasters instead of building them again.
pub const RASTER_KEEP_FRAMES: u64 = 60;

/// The old fixed raster used by sprites and levels. Eight times the svg
/// with no anti aliasing, the mip chain covers drawing it smaller.
const FIXED_SCALE: f32 = 8.0;

#[derive(Debug)]
struct Raster {
    bind:      ImageBind,
    last_used: u64,
}

/// The parsed svg tree kept next to its image so an `ImageView` can
/// rasterize it at the exact pixel size it draws, anti aliased, one
/// texture per size. Sizes not drawn for `RASTER_KEEP_FRAMES` are dropped.
#[derive(Debug)]
pub struct Svg {
    tree:    Tree,
    rasters: Mutex<HashMap<(u32, u32), Raster>>,
}

impl Svg {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self> {
        let tree = Tree::from_data(bytes, &Options::default())?;
        Ok(Self {
            tree,
            rasters: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn is_svg(bytes: &[u8]) -> bool {
        bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml")
    }

    pub(crate) fn fixed_raster(bytes: &[u8]) -> Result<TextureRawData> {
        let opt = Options {
            shape_rendering: ShapeRendering::OptimizeSpeed,
            text_rendering: TextRendering::OptimizeSpeed,
            image_rendering: ImageRendering::OptimizeSpeed,
            ..Default::default()
        };
        let tree = Tree::from_data(bytes, &opt)?;
        let original = tree.size().to_int_size();
        let width = (original.width().lossy_convert() * FIXED_SCALE).round().lossy_convert();
        let height = (original.height().lossy_convert() * FIXED_SCALE).round().lossy_convert();
        Ok(rasterize(&tree, Size::new(width, height)))
    }

    /// Marks the raster of `size` as used on `frame`, building it first
    /// when this size was never drawn or was dropped.
    pub(crate) fn touch(&self, size: Size<u32>, frame: u64) {
        let mut rasters = self.rasters.lock();
        let raster = rasters.entry((size.width, size.height)).or_insert_with(|| {
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
        let rasters = self.rasters.lock();
        let raster = rasters
            .get(&(size.width, size.height))
            .expect("svg raster drawn without being touched this frame");
        draw(raster.bind.get());
    }

    pub(crate) fn drop_stale(&self, frame: u64) {
        self.rasters
            .lock()
            .retain(|_, raster| raster.last_used + RASTER_KEEP_FRAMES >= frame);
    }

    /// Every cached raster size in pixels, in no particular order.
    pub fn raster_sizes(&self) -> Vec<Size<u32>> {
        self.rasters
            .lock()
            .keys()
            .map(|(width, height)| Size::new(*width, *height))
            .collect()
    }

    pub(crate) fn drop_stale_everywhere() {
        use crate::deps::refs::manage::DataManager;

        let frame = Window::render_frame();
        for image in Image::storage().values() {
            if let Some(svg) = &image.svg {
                svg.drop_stale(frame);
            }
        }
    }
}

fn rasterize(tree: &Tree, size: Size<u32>) -> TextureRawData {
    let size = Size::new(size.width.max(1), size.height.max(1));
    let original = tree.size();
    let scale_x = size.width.lossy_convert() / original.width();
    let scale_y = size.height.lossy_convert() / original.height();

    let mut pixmap = Pixmap::new(size.width, size.height).expect("svg raster size is not zero");
    render(
        tree,
        Transform::from_scale(scale_x, scale_y),
        &mut pixmap.as_mut(),
    );

    let data = hilen_pixels::demultiply_rgba(pixmap.pixels());

    TextureRawData {
        data,
        size,
        channels: 4,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    // A measurement, not a check. It prints how long one svg takes to
    // rasterize at each size. Run with --ignored --nocapture.
    #[test]
    #[ignore = "a measurement that prints timings, not a check"]
    fn svg_raster_time_per_size() {
        for name in ["settings.svg", "bin.svg"] {
            let bytes = std::fs::read(format!("../assets/images/{name}")).unwrap();
            let parse_started = Instant::now();
            let svg = Svg::parse(&bytes).unwrap();
            println!("{name} parse {:?}", parse_started.elapsed());
            for px in [24u32, 48, 96, 240, 512, 1024, 2048] {
                let started = Instant::now();
                let raster = rasterize(&svg.tree, Size::new(px, px));
                println!("  {px:>5}px {:?} {} bytes", started.elapsed(), raster.data.len());
            }
        }
    }

    #[test]
    fn rasterize_lands_at_the_requested_size() {
        let bytes = std::fs::read("../assets/images/bin.svg").unwrap();
        let svg = Svg::parse(&bytes).unwrap();
        let raster = rasterize(&svg.tree, Size::new(37, 91));
        assert_eq!(raster.size, Size::new(37, 91));
        assert_eq!(raster.data.len(), 37 * 91 * 4);
    }
}
