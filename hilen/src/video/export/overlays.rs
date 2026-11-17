//! The overlays of an export on its frames. An overlay turns into the 3
//! planes of the frame once, when it first shows, and every frame of its
//! time blends them in at the opacity of that moment.

use std::cell::OnceCell;

use ffmpeg_next::frame;
use hilen_pixels::{OverlayPlane, blend_plane, overlay_planes};
use log::debug;

use crate::{
    gm::{LossyConvert, flat::Size},
    video::{
        VideoOverlay,
        decoder::{Matrix, reader::PictureColor},
    },
};

struct Item {
    overlay: VideoOverlay,
    /// Made when the overlay first shows. None is an overlay with nothing
    /// inside the frame.
    planes:  OnceCell<Option<[OverlayPlane; 3]>>,
}

pub(super) struct Overlays {
    size:    Size<u32>,
    weights: [[i32; 4]; 3],
    items:   Vec<Item>,
}

/// What turns a premultiplied RGBA pixel into the samples of the file, a
/// row per plane, see `overlay_planes`. The colors of an overlay are sRGB
/// like every color of the engine, and they get the matrix and the range
/// the pictures of the file have. The inverse of `rgb` in `frames.rs`.
fn weights(color: PictureColor) -> [[i32; 4]; 3] {
    let (red, blue): (f64, f64) = match color.matrix {
        Matrix::Bt709 => (0.2126, 0.0722),
        Matrix::Bt601 => (0.299, 0.114),
        Matrix::Bt2020 => (0.2627, 0.0593),
    };
    let green = 1.0 - red - blue;
    let (luma, black, chroma) = if color.full_range {
        (1.0, 0.0, 1.0)
    } else {
        (219.0 / 255.0, 16.0, 224.0 / 255.0)
    };
    let fixed = |value: f64| -> i32 { (value * 65536.0).round().lossy_convert() };
    let to_blue = chroma / (2.0 * (1.0 - blue));
    let to_red = chroma / (2.0 * (1.0 - red));
    // The 4th weight is for the alpha, which goes up to 255 like a color.
    [
        [
            fixed(red * luma),
            fixed(green * luma),
            fixed(blue * luma),
            fixed(black / 255.0),
        ],
        [
            fixed(-red * to_blue),
            fixed(-green * to_blue),
            fixed((1.0 - blue) * to_blue),
            fixed(128.0 / 255.0),
        ],
        [
            fixed((1.0 - red) * to_red),
            fixed(-green * to_red),
            fixed(-blue * to_red),
            fixed(128.0 / 255.0),
        ],
    ]
}

impl Overlays {
    /// `size` is the size of the file, `color` says how its samples are
    /// read.
    pub(super) fn new(overlays: &[VideoOverlay], size: Size<u32>, color: PictureColor) -> Self {
        Self {
            size,
            weights: weights(color),
            items: overlays
                .iter()
                .map(|overlay| Item {
                    overlay: overlay.clone(),
                    planes:  OnceCell::new(),
                })
                .collect(),
        }
    }

    /// Whether a frame at this second of the list has an overlay on it.
    pub(super) fn shows(&self, seconds: f64) -> bool {
        self.items.iter().any(|item| item.overlay.opacity(seconds) > 0.0)
    }

    /// Draws every overlay that shows at this second onto `frame`, in the
    /// order of the list. `frame` is in `PIXEL` at the size of the file.
    pub(super) fn draw(&mut self, frame: &mut frame::Video, seconds: f64) {
        let size = (
            usize::try_from(self.size.width).expect("a frame width fits usize"),
            usize::try_from(self.size.height).expect("a frame height fits usize"),
        );
        for (index, item) in self.items.iter().enumerate() {
            let opacity: u32 = (item.overlay.opacity(seconds) * 256.0).round().lossy_convert();
            if opacity == 0 {
                continue;
            }
            let planes = item.planes.get_or_init(|| {
                let layer = item.overlay.layer(self.size)?;
                debug!(
                    "video export: overlay {index} is {} by {} pixels at {}, {}",
                    layer.width,
                    layer.rgba.len() / 4 / layer.width.max(1),
                    layer.left,
                    layer.top
                );
                overlay_planes(
                    &layer.rgba,
                    layer.width,
                    (layer.left, layer.top),
                    size,
                    &self.weights,
                )
            });
            let Some(planes) = planes else {
                continue;
            };
            for (plane, samples) in planes.iter().enumerate() {
                let stride = frame.stride(plane);
                blend_plane(frame.data_mut(plane), stride, samples, opacity);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use ffmpeg_next::frame;

    use crate::{
        gm::{LossyConvert, color::WHITE, flat::Size},
        video::{
            VideoOverlay,
            decoder::{Matrix, Transfer, reader::PictureColor},
            export::{overlays::Overlays, writer::PIXEL},
        },
    };

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 48;
    /// The luma and the chroma the test frame is filled with, a gray.
    const GRAY: [u8; 3] = [100, 128, 128];
    /// White and red in BT.709 limited range.
    const WHITE_SAMPLES: [u8; 3] = [235, 128, 128];
    const RED_SAMPLES: [u8; 3] = [63, 102, 240];

    fn gray_frame(width: u32, height: u32) -> frame::Video {
        let mut frame = frame::Video::new(PIXEL, width, height);
        for (plane, value) in GRAY.into_iter().enumerate() {
            frame.data_mut(plane).fill(value);
        }
        frame
    }

    fn overlays(list: &[VideoOverlay], width: u32, height: u32) -> Overlays {
        let color = PictureColor {
            full_range: false,
            matrix:     Matrix::Bt709,
            transfer:   Transfer::Sdr,
        };
        Overlays::new(list, Size::new(width, height), color)
    }

    /// The 3 samples of a pixel of the frame.
    fn pixel(frame: &frame::Video, x: usize, y: usize) -> [u8; 3] {
        [
            frame.data(0)[y * frame.stride(0) + x],
            frame.data(1)[y / 2 * frame.stride(1) + x / 2],
            frame.data(2)[y / 2 * frame.stride(2) + x / 2],
        ]
    }

    fn near(have: [u8; 3], want: [u8; 3]) -> bool {
        have.iter().zip(want).all(|(have, want)| have.abs_diff(want) <= 1)
    }

    /// A picture of 1 color, `side` pixels wide and high.
    fn square(side: u32, rgba: [u8; 4]) -> VideoOverlay {
        let pixels = usize::try_from(side * side).expect("a small picture");
        VideoOverlay::picture_rgba(rgba.repeat(pixels), side, side).expect("a picture")
    }

    /// A red square of half the frame height in the middle: 24 by 24 pixels
    /// from 20, 12. It is scaled up from 4 by 4 pixels.
    #[test]
    fn a_picture_covers_its_rect_and_nothing_else() {
        let overlay = square(4, [255, 0, 0, 255]).with_size(0.5);
        let rect = overlay.rect(Size::new(WIDTH.lossy_convert(), HEIGHT.lossy_convert()));
        assert_eq!(
            (rect.x(), rect.y(), rect.width(), rect.height()),
            (20.0, 12.0, 24.0, 24.0)
        );

        let mut frame = gray_frame(WIDTH, HEIGHT);
        let mut overlays = overlays(&[overlay], WIDTH, HEIGHT);
        assert!(overlays.shows(0.0));
        overlays.draw(&mut frame, 0.0);
        for y in 0..48 {
            for x in 0..64 {
                let inside = (20..44).contains(&x) && (12..36).contains(&y);
                let have = pixel(&frame, x, y);
                if inside {
                    assert!(near(have, RED_SAMPLES), "{have:?} at {x}, {y} inside");
                } else {
                    assert_eq!(have, GRAY, "at {x}, {y} outside");
                }
            }
        }
    }

    /// The left half of the picture is green with alpha 0, the right half
    /// is white with alpha 1. The picture has the size of its rect, so no
    /// scaling mixes the halves.
    #[test]
    fn alpha_0_changes_nothing_and_alpha_1_replaces() {
        let mut rgba = Vec::new();
        for index in 0..24 * 24 {
            let left = index % 24 < 12;
            rgba.extend_from_slice(if left {
                &[0, 255, 0, 0]
            } else {
                &[255, 255, 255, 255]
            });
        }
        let overlay = VideoOverlay::picture_rgba(rgba, 24, 24).expect("a picture").with_size(0.5);
        let mut frame = gray_frame(WIDTH, HEIGHT);
        overlays(&[overlay], WIDTH, HEIGHT).draw(&mut frame, 0.0);
        for y in 12..36 {
            for x in 20..44 {
                let want = if x < 32 { GRAY } else { WHITE_SAMPLES };
                assert_eq!(pixel(&frame, x, y), want, "at {x}, {y}");
            }
        }
    }

    /// A picture half outside of the frame draws the part that is inside.
    #[test]
    fn a_picture_over_the_edge_is_cut() {
        let overlay = square(24, [255, 255, 255, 255]).with_size(0.5).with_center(0.0, 0.0);
        let mut frame = gray_frame(WIDTH, HEIGHT);
        overlays(&[overlay], WIDTH, HEIGHT).draw(&mut frame, 0.0);
        for (x, y, want) in [
            (0, 0, WHITE_SAMPLES),
            (11, 11, WHITE_SAMPLES),
            (12, 11, GRAY),
            (11, 12, GRAY),
            (63, 47, GRAY),
        ] {
            assert_eq!(pixel(&frame, x, y), want, "at {x}, {y}");
        }
    }

    /// From 1 to 3 seconds with a fade in of 1 second: nothing before the
    /// start and from the end on, half as strong in the middle of the fade.
    #[test]
    fn an_overlay_shows_in_its_time_and_fades() {
        let overlay = square(24, [255, 255, 255, 255])
            .with_size(0.5)
            .with_time(1.0, 3.0)
            .with_fade(1.0, 0.0);
        let mut overlays = overlays(&[overlay], WIDTH, HEIGHT);
        let mut at = |seconds: f64| {
            let mut frame = gray_frame(WIDTH, HEIGHT);
            overlays.draw(&mut frame, seconds);
            (pixel(&frame, 32, 24), pixel(&frame, 2, 2))
        };
        for seconds in [0.0, 0.99, 3.0, 4.0] {
            assert_eq!(at(seconds), (GRAY, GRAY), "at {seconds} s");
        }
        assert_eq!(at(2.0), (WHITE_SAMPLES, GRAY));
        assert_eq!(at(2.99), (WHITE_SAMPLES, GRAY));
        // Half way from the gray 100 to the white 235.
        let (half, outside) = at(1.5);
        assert!(near(half, [168, 128, 128]), "{half:?} in the middle of the fade");
        assert_eq!(outside, GRAY);
        let (quarter, _) = at(1.25);
        assert!(
            near(quarter, [134, 128, 128]),
            "{quarter:?} a quarter into the fade"
        );

        assert!(!overlays.shows(0.5) && !overlays.shows(3.0));
        assert!(overlays.shows(1.5) && overlays.shows(2.5));
    }

    /// A white text draws inside the rect it reports, with full white in
    /// its stems, and leaves the rest of the frame alone.
    #[test]
    fn a_text_draws_inside_its_rect() {
        let (width, height) = (320_u32, 180_u32);
        let overlay = VideoOverlay::text("Hilen", WHITE).with_size(0.3).with_center(0.5, 0.4);
        let rect = overlay.rect(Size::new(width.lossy_convert(), height.lossy_convert()));
        let mut frame = gray_frame(width, height);
        overlays(&[overlay], width, height).draw(&mut frame, 0.0);

        let (mut drawn, mut white) = (0, 0);
        for y in 0..180_usize {
            for x in 0..320_usize {
                let luma = frame.data(0)[y * frame.stride(0) + x];
                if luma == GRAY[0] {
                    continue;
                }
                drawn += 1;
                white += usize::from(luma == WHITE_SAMPLES[0]);
                let (at_x, at_y): (f32, f32) = (x.lossy_convert(), y.lossy_convert());
                assert!(
                    at_x >= rect.x() - 2.0
                        && at_x <= rect.max_x() + 2.0
                        && at_y >= rect.y() - 2.0
                        && at_y <= rect.max_y() + 2.0,
                    "the text drew at {x}, {y}, outside of {rect:?}"
                );
            }
        }
        assert!(drawn > 500, "the text drew {drawn} pixels");
        assert!(white > 200, "the text has {white} pixels of full white");
    }

    /// A text with nothing to draw and an overlay with no size cost a
    /// frame nothing.
    #[test]
    fn an_empty_overlay_draws_nothing() {
        let list = [
            VideoOverlay::text("  ", WHITE),
            VideoOverlay::text("", WHITE),
            square(4, [255, 0, 0, 255]).with_size(0.0),
        ];
        let mut frame = gray_frame(WIDTH, HEIGHT);
        overlays(&list, WIDTH, HEIGHT).draw(&mut frame, 0.0);
        for y in 0..48 {
            for x in 0..64 {
                assert_eq!(pixel(&frame, x, y), GRAY, "at {x}, {y}");
            }
        }
    }
}
