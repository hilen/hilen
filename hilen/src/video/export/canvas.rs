//! A decoded picture as a frame of the export: scaled to fit the size of the
//! file with its shape kept, in the middle, with black around it.

use ffmpeg_next::{Error, format::Pixel, frame, software::scaling};

use crate::{
    gm::{LossyConvert, flat::Size},
    video::export::writer::PIXEL,
};

/// The luma and the chroma of black in limited range.
const BLACK: [u8; 3] = [16, 128, 128];

/// What a scaler was built for.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Scaling {
    format: Pixel,
    from:   Size<u32>,
    to:     Size<u32>,
}

pub(super) struct Canvas {
    size:   Size<u32>,
    built:  Option<Scaling>,
    scaler: Option<scaling::Context>,
}

/// The biggest size with the shape of `picture` that fits into `canvas`,
/// both sides even, since the chroma planes have half the size.
fn fit(picture: Size<u32>, canvas: Size<u32>) -> Size<u32> {
    let scale = (f64::from(canvas.width) / f64::from(picture.width.max(1)))
        .min(f64::from(canvas.height) / f64::from(picture.height.max(1)));
    let side = |full: u32, limit: u32| -> u32 {
        let scaled: u32 = (f64::from(full) * scale).round().lossy_convert();
        (scaled / 2 * 2).clamp(2, limit)
    };
    Size::new(
        side(picture.width, canvas.width),
        side(picture.height, canvas.height),
    )
}

impl Canvas {
    /// `size` has even sides.
    pub(super) fn new(size: Size<u32>) -> Self {
        Self {
            size,
            built: None,
            scaler: None,
        }
    }

    /// A fresh frame of the file with this picture on it. `source` is in
    /// system memory. The samples are scaled, not converted: a 10 bit
    /// picture is cut to 8 bits, the matrix and the range stay.
    pub(super) fn frame(&mut self, source: &frame::Video) -> Result<frame::Video, Error> {
        let from = Size::new(source.width(), source.height());
        let scaling = Scaling {
            format: source.format(),
            from,
            to: fit(from, self.size),
        };
        if self.built != Some(scaling) {
            self.scaler = Some(scaling::Context::get(
                scaling.format,
                from.width,
                from.height,
                PIXEL,
                scaling.to.width,
                scaling.to.height,
                scaling::Flags::BICUBIC | scaling::Flags::ACCURATE_RND,
            )?);
            self.built = Some(scaling);
        }
        let scaler = self.scaler.as_mut().expect("the scaler was just made");
        let mut fitted = frame::Video::empty();
        scaler.run(source, &mut fitted)?;
        if scaling.to == self.size {
            return Ok(fitted);
        }

        let mut canvas = frame::Video::new(PIXEL, self.size.width, self.size.height);
        let left = (self.size.width - scaling.to.width) / 2;
        let top = (self.size.height - scaling.to.height) / 2;
        for (plane, black) in BLACK.into_iter().enumerate() {
            // The chroma planes have half the size, and so half the offset.
            let halved = u32::from(plane > 0);
            let side = |pixels: u32| usize::try_from(pixels >> halved).expect("a picture side fits usize");
            let (width, height) = (side(scaling.to.width), side(scaling.to.height));
            let (left, top) = (side(left), side(top));
            let (from_stride, to_stride) = (fitted.stride(plane), canvas.stride(plane));
            let to = canvas.data_mut(plane);
            to.fill(black);
            let rows = fitted.data(plane).chunks(from_stride).take(height);
            for (row, line) in rows.enumerate() {
                let at = (top + row) * to_stride + left;
                to[at..at + width].copy_from_slice(&line[..width]);
            }
        }
        Ok(canvas)
    }
}

#[cfg(test)]
mod test {
    use crate::{gm::flat::Size, video::export::canvas::fit};

    #[test]
    fn a_picture_fits_the_canvas_with_even_sides() {
        let canvas = Size::new(1920, 1080);
        assert_eq!(fit(Size::new(1920, 1080), canvas), canvas);
        assert_eq!(fit(Size::new(3840, 2160), canvas), canvas);
        assert_eq!(fit(Size::new(640, 360), canvas), canvas);
        // A square and a tall picture get bars at the sides.
        assert_eq!(fit(Size::new(128, 128), canvas), Size::new(1080, 1080));
        assert_eq!(fit(Size::new(1080, 1920), canvas), Size::new(608, 1080));
        // A wide one gets bars at the top and the bottom.
        assert_eq!(fit(Size::new(2000, 500), canvas), Size::new(1920, 480));
        assert_eq!(fit(Size::new(128, 128), Size::new(128, 72)), Size::new(72, 72));
    }
}
