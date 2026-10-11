//! A picture or a text that an export draws over the frames of its file,
//! see `export/overlays.rs`. The place and the size are parts of the frame,
//! so 1 overlay fits a file of any size and the preview of an app.

mod text;

use std::{
    fmt::{Debug, Formatter},
    path::Path,
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use hilen_pixels::premultiply_rgba;
use image::{
    RgbaImage,
    imageops::{FilterType, resize},
    load_from_memory,
};

use crate::{
    filesystem::read_bytes,
    gm::{
        LossyConvert,
        color::Color,
        flat::{Point, Rect, Size},
    },
};

/// The pixels of a picture, RGBA with straight alpha.
#[derive(PartialEq, Eq)]
struct Pixels {
    width:  u32,
    height: u32,
    rgba:   Vec<u8>,
}

impl Debug for Pixels {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} by {} pixels", self.width, self.height)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Content {
    Picture(Arc<Pixels>),
    Text { text: String, color: Color },
}

/// Pixels ready to go onto a frame: premultiplied RGBA, `width` wide, with
/// the place of the top left pixel in the frame. The place can be outside.
pub(crate) struct Layer {
    pub left:  i64,
    pub top:   i64,
    pub width: usize,
    pub rgba:  Vec<u8>,
}

/// A picture or a text drawn over the video of an export, in
/// `VideoExportSettings::overlays`. A later overlay of the list is drawn
/// over an earlier one.
///
/// ```ignore
/// let logo = VideoOverlay::picture("logo.png")?
///     .with_center(0.5, 0.4)
///     .with_size(0.3)
///     .with_time(10.0, 15.0)
///     .with_fade(0.5, 0.5);
/// let name = VideoOverlay::text("Railroads", WHITE)
///     .with_center(0.5, 0.65)
///     .with_size(0.08)
///     .with_time(10.0, 15.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VideoOverlay {
    content:      Content,
    /// The middle of the overlay as parts of the frame: 0 is the left or
    /// the top edge, 1 is the right or the bottom edge.
    pub center:   Point,
    /// For a picture its height as a part of the frame height, the width
    /// follows from the shape of the picture. For a text the text size as
    /// a part of the frame height, what `Label::set_text_size` takes in a
    /// preview.
    pub size:     f32,
    /// The first second of the list the overlay shows at.
    pub start:    f64,
    /// The second of the list the overlay is gone at.
    pub end:      f64,
    /// Seconds the overlay takes to get from clear to full after `start`.
    pub fade_in:  f64,
    /// Seconds the overlay takes to get from full to clear before `end`.
    pub fade_out: f64,
}

impl VideoOverlay {
    fn new(content: Content) -> Self {
        Self {
            content,
            center: Point::new(0.5, 0.5),
            size: 0.1,
            start: 0.0,
            end: f64::INFINITY,
            fade_in: 0.0,
            fade_out: 0.0,
        }
    }

    /// A picture from a file, a PNG with alpha for example. The file is
    /// read and decoded here, on the thread that calls. The picture keeps
    /// its alpha. It shows in the middle of the frame for the whole list,
    /// at a tenth of the frame height, until the calls below say more.
    pub fn picture(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = read_bytes(path).with_context(|| format!("the picture {}", path.display()))?;
        let picture = load_from_memory(&bytes)
            .with_context(|| format!("the picture {}", path.display()))?
            .into_rgba8();
        let (width, height) = picture.dimensions();
        Self::picture_rgba(picture.into_raw(), width, height)
    }

    /// A picture from RGBA pixels with straight alpha, row after row.
    pub fn picture_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self> {
        let pixels = u64::from(width) * u64::from(height);
        if pixels == 0 || u64::try_from(rgba.len())? != pixels * 4 {
            bail!(
                "{} bytes are not a picture of {width} by {height} pixels",
                rgba.len()
            );
        }
        Ok(Self::new(Content::Picture(Arc::new(Pixels {
            width,
            height,
            rgba,
        }))))
    }

    /// A text in the font a `Label` has by default, Roboto. A line break in
    /// the text starts a new line, every line stands in the middle. The
    /// alpha of the color counts.
    pub fn text(text: impl Into<String>, color: impl Into<Color>) -> Self {
        Self::new(Content::Text {
            text:  text.into(),
            color: color.into(),
        })
    }

    /// The same overlay with its middle at this place, see `center`.
    #[must_use]
    pub fn with_center(mut self, x: f32, y: f32) -> Self {
        self.center = Point::new(x, y);
        self
    }

    /// The same overlay at this size, see `size`.
    #[must_use]
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// The same overlay shown from `start` up to `end`, seconds of the list.
    #[must_use]
    pub fn with_time(mut self, start: f64, end: f64) -> Self {
        self.start = start;
        self.end = end;
        self
    }

    /// The same overlay with a fade at its start and at its end, seconds.
    #[must_use]
    pub fn with_fade(mut self, fade_in: f64, fade_out: f64) -> Self {
        self.fade_in = fade_in;
        self.fade_out = fade_out;
        self
    }

    /// How strong the overlay is at a second of the list: 0 before `start`
    /// and from `end` on, 1 between the fades, and in a fade the part of it
    /// that has passed. A preview sets this as the alpha of its view.
    pub fn opacity(&self, seconds: f64) -> f32 {
        if seconds.is_nan() || seconds < self.start || seconds >= self.end {
            return 0.0;
        }
        let part = |passed: f64, fade: f64| if fade > 0.0 { (passed / fade).min(1.0) } else { 1.0 };
        let strength = part(seconds - self.start, self.fade_in).min(part(self.end - seconds, self.fade_out));
        strength.lossy_convert()
    }

    /// The rect the overlay covers in a frame of this size, in the units of
    /// that size. An export asks with the pixels of the file. A preview
    /// asks with the size of its video picture and gets the frame for its
    /// `ImageView`. For a text the rect is the box of its lines: a `Label`
    /// with its middle on the middle of this rect, the size of `text_size`
    /// and the default alignment draws the same lines. Make the label wider
    /// than the rect, a label cuts a text that does not fit its width.
    pub fn rect(&self, frame: Size) -> Rect {
        let size = match &self.content {
            Content::Picture(pixels) => {
                let height = self.size * frame.height;
                let width: f32 = pixels.width.lossy_convert();
                let picture_height: f32 = pixels.height.lossy_convert();
                Size::new(height * width / picture_height, height)
            }
            Content::Text { text, .. } => text::measure(text, self.text_size(frame)),
        };
        Rect::new(
            self.center.x * frame.width - size.width / 2.0,
            self.center.y * frame.height - size.height / 2.0,
            size.width,
            size.height,
        )
    }

    /// The text size of a text overlay in a frame of this size, what a
    /// `Label` of a preview takes in `set_text_size`.
    pub fn text_size(&self, frame: Size) -> f32 {
        self.size * frame.height
    }

    /// The pixels of the overlay for a frame of this size. None when it has
    /// nothing to draw, a text of spaces or a size of 0.
    pub(crate) fn layer(&self, frame: Size<u32>) -> Option<Layer> {
        let frame = Size::new(frame.width.lossy_convert(), frame.height.lossy_convert());
        let rect = self.rect(frame);
        if !(rect.width().is_finite()
            && rect.height().is_finite()
            && rect.x().is_finite()
            && rect.y().is_finite())
        {
            return None;
        }
        match &self.content {
            Content::Picture(pixels) => {
                let width: u32 = rect.width().round().max(1.0).lossy_convert();
                let height: u32 = rect.height().round().max(1.0).lossy_convert();
                if self.size <= 0.0 {
                    return None;
                }
                // Scaled with the colors times their alpha, or the color
                // of a clear pixel would bleed into the edge next to it.
                let mut rgba = pixels.rgba.clone();
                premultiply_rgba(&mut rgba);
                if (width, height) != (pixels.width, pixels.height) {
                    let picture = RgbaImage::from_raw(pixels.width, pixels.height, rgba)?;
                    rgba = resize(&picture, width, height, FilterType::CatmullRom).into_raw();
                }
                Some(Layer {
                    left: rect.x().round().lossy_convert(),
                    top: rect.y().round().lossy_convert(),
                    width: usize::try_from(width).ok()?,
                    rgba,
                })
            }
            Content::Text { text, color } => {
                let middle = Point::new(self.center.x * frame.width, self.center.y * frame.height);
                text::raster(text, self.text_size(frame), middle, *color)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::env::temp_dir;

    use image::RgbaImage;

    use crate::{
        gm::{
            color::WHITE,
            flat::{Rect, Size},
        },
        video::VideoOverlay,
    };

    fn near(have: f32, want: f32) -> bool {
        (have - want).abs() < 0.001
    }

    #[test]
    fn a_picture_rect_follows_the_frame_and_the_shape() {
        // 40 by 20 pixels, twice as wide as high.
        let overlay = VideoOverlay::picture_rgba(vec![255; 40 * 20 * 4], 40, 20)
            .expect("a picture")
            .with_center(0.25, 0.5)
            .with_size(0.5);
        assert_eq!(
            overlay.rect(Size::new(1920.0, 1080.0)),
            Rect::new(480.0 - 540.0, 270.0, 1080.0, 540.0)
        );
        // A preview of another size gets the same parts of its frame.
        assert_eq!(
            overlay.rect(Size::new(640.0, 360.0)),
            Rect::new(160.0 - 180.0, 90.0, 360.0, 180.0)
        );
        assert!(VideoOverlay::picture_rgba(vec![0; 10], 2, 2).is_err());
    }

    /// A PNG file with alpha gives the pixels that were saved, alpha
    /// included.
    #[test]
    fn a_png_file_keeps_its_alpha() {
        let rgba: Vec<u8> = (0..6 * 4_u8)
            .flat_map(|index| [index * 10, 255 - index, 7, index * 11])
            .collect();
        let path = temp_dir().join("hilen-video-overlay-alpha.png");
        RgbaImage::from_raw(6, 4, rgba.clone())
            .expect("the pixels fill the picture")
            .save(&path)
            .expect("the png is written");
        let from_file = VideoOverlay::picture(&path).expect("the png is read");
        let from_pixels = VideoOverlay::picture_rgba(rgba, 6, 4).expect("a picture");
        assert_eq!(from_file, from_pixels);
        assert!(VideoOverlay::picture(temp_dir().join("hilen-no-such-picture.png")).is_err());
    }

    #[test]
    fn a_text_rect_grows_with_the_frame_and_the_lines() {
        let one = VideoOverlay::text("Railroads", WHITE).with_size(0.1);
        let small = one.rect(Size::new(640.0, 360.0));
        let big = one.rect(Size::new(1280.0, 720.0));
        assert!(small.width() > 36.0 * 3.0, "9 letters are {} wide", small.width());
        // The line of Roboto is 1.17 of its text size.
        assert!(near(small.height() / 36.0, 1.171_875), "{}", small.height());
        assert!(near(big.width() / small.width(), 2.0) && near(big.height() / small.height(), 2.0));
        assert!(near(small.center().x, 320.0) && near(small.center().y, 180.0));
        assert!(near(one.text_size(Size::new(640.0, 360.0)), 36.0));

        let two = VideoOverlay::text("Railroads\nmade in kutreel", WHITE).with_size(0.1);
        let both = two.rect(Size::new(640.0, 360.0));
        assert!(both.height() > small.height() * 1.9 && both.width() > small.width());
    }

    #[test]
    fn the_opacity_follows_the_time_and_the_fades() {
        let overlay = VideoOverlay::text("a", WHITE).with_time(2.0, 6.0).with_fade(1.0, 2.0);
        for (seconds, expected) in [
            (0.0, 0.0),
            (1.99, 0.0),
            (2.0, 0.0),
            (2.5, 0.5),
            (3.0, 1.0),
            (4.0, 1.0),
            (5.0, 0.5),
            (5.5, 0.25),
            (6.0, 0.0),
            (9.0, 0.0),
        ] {
            let opacity = overlay.opacity(seconds);
            assert!(
                near(opacity, expected),
                "{opacity} at {seconds} s, not {expected}"
            );
        }
        // No fade: full from the start, gone at the end.
        let hard = VideoOverlay::text("a", WHITE).with_time(2.0, 6.0);
        assert!(near(hard.opacity(2.0), 1.0) && near(hard.opacity(5.99), 1.0));
        assert!(near(hard.opacity(6.0), 0.0));
        // An overlay with no time shows for the whole list.
        assert!(near(VideoOverlay::text("a", WHITE).opacity(1000.0), 1.0));
    }
}
