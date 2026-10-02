use crate::gm::flat::{Rect, Size};

/// The direction an image is cut along, see `ImageView::set_cut`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutAxis {
    /// The left and right ends stay, the middle is cut in width.
    Horizontal,
    /// The top and bottom ends stay, the middle is cut in height.
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ImageCut {
    pub axis:  CutAxis,
    /// The length of the first end, in pixels of the image.
    pub start: f32,
    /// The length of the last end, in pixels of the image.
    pub end:   f32,
}

/// 1 of the 3 parts the image is cut into, where it draws along the cut axis
/// and what part of the image it shows.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Span {
    position:    f32,
    length:      f32,
    uv_position: f32,
    uv_length:   f32,
}

impl ImageCut {
    /// How many points 1 pixel of the image takes. The image scales as a
    /// whole to the size of the view across the cut axis.
    fn points_per_pixel(&self, frame: Rect, image: Size) -> f32 {
        let (view, image) = match self.axis {
            CutAxis::Horizontal => (frame.height(), image.height),
            CutAxis::Vertical => (frame.width(), image.width),
        };
        if image > 0.0 { view / image } else { 0.0 }
    }

    /// The size the whole image has at the scale of this view, the size an
    /// svg rasterizes at.
    pub(crate) fn scaled_size(&self, frame: Rect, image: Size) -> Size {
        image * self.points_per_pixel(frame, image)
    }

    /// The parts to draw, each a frame and its UV rect. The cuts land on whole
    /// physical pixels, because the image shader fades the last pixel of every
    /// quad edge and 2 faded edges on 1 pixel would show as a seam.
    pub(crate) fn parts(&self, frame: Rect, image: Size, scale: f32) -> Vec<(Rect, Rect)> {
        let points_per_pixel = self.points_per_pixel(frame, image);

        let spans = match self.axis {
            CutAxis::Horizontal => self.spans(frame.x(), frame.width(), image.width, points_per_pixel, scale),
            CutAxis::Vertical => self.spans(frame.y(), frame.height(), image.height, points_per_pixel, scale),
        };

        spans
            .into_iter()
            .filter(|span| span.length > 0.0)
            .map(|span| match self.axis {
                CutAxis::Horizontal => (
                    Rect::new(span.position, frame.y(), span.length, frame.height()),
                    Rect::new(span.uv_position, 0.0, span.uv_length, 1.0),
                ),
                CutAxis::Vertical => (
                    Rect::new(frame.x(), span.position, frame.width(), span.length),
                    Rect::new(0.0, span.uv_position, 1.0, span.uv_length),
                ),
            })
            .collect()
    }

    fn spans(&self, origin: f32, length: f32, image: f32, points_per_pixel: f32, scale: f32) -> [Span; 3] {
        let start = self.start.clamp(0.0, image);
        let end = self.end.clamp(0.0, image - start);
        let middle = image - start - end;

        let mut start_length = start * points_per_pixel;
        let mut end_length = end * points_per_pixel;

        // A view shorter than its 2 ends squeezes both and has no middle.
        let ends = start_length + end_length;
        if ends > length && ends > 0.0 {
            let squeeze = length / ends;
            start_length *= squeeze;
            end_length *= squeeze;
        }

        let snap = |position: f32| (position * scale).round() / scale;
        let first_cut = snap(origin + start_length).clamp(origin, origin + length);
        let second_cut = snap(origin + length - end_length).clamp(first_cut, origin + length);

        // The middle shows as many image pixels as fit at the scale of the
        // ends, taken from the side of the first end, so only the last end
        // meets a cut. A view longer than the image has no more pixels to
        // show and stretches the whole middle.
        let middle_length = second_cut - first_cut;
        let shown = if points_per_pixel > 0.0 {
            (middle_length / points_per_pixel).min(middle)
        } else {
            middle
        };

        let uv = |pixels: f32| if image > 0.0 { pixels / image } else { 0.0 };

        [
            Span {
                position:    origin,
                length:      first_cut - origin,
                uv_position: 0.0,
                uv_length:   uv(start),
            },
            Span {
                position:    first_cut,
                length:      middle_length,
                uv_position: uv(start),
                uv_length:   uv(shown),
            },
            Span {
                position:    second_cut,
                length:      origin + length - second_cut,
                uv_position: 1.0 - uv(end),
                uv_length:   uv(end),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::{CutAxis, ImageCut};
    use crate::gm::flat::{Rect, Size};

    const IMAGE: Size = Size::new(600.0, 100.0);

    fn cut() -> ImageCut {
        ImageCut {
            axis:  CutAxis::Horizontal,
            start: 50.0,
            end:   100.0,
        }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn a_short_view_shows_a_short_piece_of_the_middle() {
        // Half the image height, so 1 image pixel is half a point.
        let parts = cut().parts(Rect::new(10.0, 20.0, 200.0, 50.0), IMAGE, 1.0);

        assert_eq!(parts.len(), 3);
        // The ends keep their shape at the scale of the view.
        assert_eq!(parts[0].0, Rect::new(10.0, 20.0, 25.0, 50.0));
        assert_eq!(parts[2].0, Rect::new(160.0, 20.0, 50.0, 50.0));
        // 125 points of middle show 250 image pixels, not the whole 450.
        assert_eq!(parts[1].0, Rect::new(35.0, 20.0, 125.0, 50.0));
        assert!(close(parts[1].1.x(), 50.0 / 600.0));
        assert!(close(parts[1].1.width(), 250.0 / 600.0));
        // Nothing is stretched: every part has the same points per uv.
        assert!(close(parts[0].0.width() / parts[0].1.width(), 300.0));
        assert!(close(parts[1].0.width() / parts[1].1.width(), 300.0));
        assert!(close(parts[2].0.width() / parts[2].1.width(), 300.0));
    }

    #[test]
    fn a_bigger_view_has_bigger_ends() {
        let small = cut().parts(Rect::new(0.0, 0.0, 300.0, 50.0), IMAGE, 1.0);
        let big = cut().parts(Rect::new(0.0, 0.0, 600.0, 100.0), IMAGE, 1.0);

        assert_eq!(small[0].0.width(), 25.0);
        assert_eq!(big[0].0.width(), 50.0);
        assert_eq!(small[2].0.width(), 50.0);
        assert_eq!(big[2].0.width(), 100.0);
    }

    #[test]
    fn a_view_as_long_as_the_image_shows_all_of_it() {
        let parts = cut().parts(Rect::new(0.0, 0.0, 600.0, 100.0), IMAGE, 1.0);

        assert!(close(parts[1].1.width(), 450.0 / 600.0));
        assert!(close(parts[1].1.max_x(), parts[2].1.x()));
    }

    #[test]
    fn a_view_longer_than_the_image_stretches_the_middle() {
        let parts = cut().parts(Rect::new(0.0, 0.0, 1000.0, 100.0), IMAGE, 1.0);

        assert_eq!(parts[1].0.width(), 850.0);
        assert!(close(parts[1].1.width(), 450.0 / 600.0));
    }

    #[test]
    fn a_view_shorter_than_its_ends_has_no_middle() {
        let parts = cut().parts(Rect::new(0.0, 0.0, 60.0, 100.0), IMAGE, 1.0);

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].0, Rect::new(0.0, 0.0, 20.0, 100.0));
        assert_eq!(parts[1].0, Rect::new(20.0, 0.0, 40.0, 100.0));
    }

    #[test]
    fn cuts_land_on_physical_pixels() {
        let parts = cut().parts(Rect::new(10.3, 0.0, 200.0, 50.0), IMAGE, 2.0);

        // 10.3 + 25 = 35.3, the nearest half point is 35.5.
        assert_eq!(parts[1].0.x(), 35.5);
        assert_eq!(parts[2].0.x(), 160.5);
    }

    #[test]
    fn a_vertical_cut_keeps_the_top_and_the_bottom() {
        let cut = ImageCut {
            axis:  CutAxis::Vertical,
            start: 100.0,
            end:   50.0,
        };
        let parts = cut.parts(Rect::new(0.0, 0.0, 50.0, 200.0), Size::new(100.0, 600.0), 1.0);

        assert_eq!(parts[0].0, Rect::new(0.0, 0.0, 50.0, 50.0));
        assert_eq!(parts[1].0, Rect::new(0.0, 50.0, 50.0, 125.0));
        assert_eq!(parts[2].0, Rect::new(0.0, 175.0, 50.0, 25.0));
        assert!(close(parts[1].1.height(), 250.0 / 600.0));
    }
}
