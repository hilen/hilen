//! Where the photo of an `ImageCropView` stands under its mask. It is kept
//! in pixels of the photo, so a new size of the view keeps the same cut.

use crate::gm::flat::{Point, Rect, Size};

/// How far a player can zoom in, past the zoom where the short side of the
/// photo just fills the mask.
pub(crate) const MAX_ZOOM: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CropFrame {
    /// The photo, in pixels.
    pub picture: Size,
    /// 1 when the short side of the photo fills the mask.
    pub zoom:    f32,
    /// The pixel of the photo in the middle of the mask.
    pub focus:   Point,
}

impl Default for CropFrame {
    fn default() -> Self {
        Self::new(Size::new(0.0, 0.0))
    }
}

impl CropFrame {
    /// The whole short side of the photo in the mask, the middle of it in
    /// the middle, the way every photo opens.
    pub fn new(picture: Size) -> Self {
        Self {
            picture,
            zoom: 1.0,
            focus: Point::new(picture.width / 2.0, picture.height / 2.0),
        }
    }

    /// Points of the view for 1 pixel of the photo, with a mask `side`
    /// points across.
    pub fn scale(&self, side: f32) -> f32 {
        let short = self.short_side();
        if short <= 0.0 {
            return 0.0;
        }
        side / short * self.zoom
    }

    /// Moves the photo by `delta` points of the view.
    pub fn pan(&mut self, delta: Point, side: f32) {
        let scale = self.scale(side);
        if scale <= 0.0 {
            return;
        }
        self.focus -= delta / scale;
        self.clamp();
    }

    /// Zooms by `factor` and keeps the pixel under `at` where it is. `at`
    /// is in points from the middle of the mask.
    pub fn zoom_at(&mut self, at: Point, factor: f32, side: f32) {
        let scale = self.scale(side);
        if scale <= 0.0 || !factor.is_finite() {
            return;
        }
        let pixel = self.focus + at / scale;
        self.zoom = (self.zoom * factor).clamp(1.0, MAX_ZOOM);
        self.focus = pixel - at / self.scale(side);
        self.clamp();
    }

    /// Sets the zoom and keeps the middle of the mask where it is.
    pub fn set_zoom(&mut self, zoom: f32, side: f32) {
        if self.zoom > 0.0 {
            self.zoom_at(Point::default(), zoom / self.zoom, side);
        }
    }

    /// The cut, a square in pixels of the photo. It is always inside the
    /// photo.
    pub fn cut(&self) -> Rect {
        let half = self.half_cut();
        Rect::new(self.focus.x - half, self.focus.y - half, half * 2.0, half * 2.0)
    }

    /// Where the whole photo is drawn, in points from the middle of the
    /// mask.
    pub fn picture_rect(&self, side: f32) -> Rect {
        let scale = self.scale(side);
        Rect::new(
            -self.focus.x * scale,
            -self.focus.y * scale,
            self.picture.width * scale,
            self.picture.height * scale,
        )
    }

    fn short_side(&self) -> f32 {
        self.picture.width.min(self.picture.height)
    }

    /// Half the side of the cut in pixels. It does not depend on the size
    /// of the view, only on the zoom.
    fn half_cut(&self) -> f32 {
        self.short_side() / self.zoom / 2.0
    }

    /// The mask never shows anything past the edge of the photo.
    fn clamp(&mut self) {
        let half = self.half_cut();
        self.focus.x = self.focus.x.clamp(half, (self.picture.width - half).max(half));
        self.focus.y = self.focus.y.clamp(half, (self.picture.height - half).max(half));
    }
}

#[cfg(test)]
mod test {
    use super::{CropFrame, MAX_ZOOM};
    use crate::gm::flat::{Point, Rect, Size};

    const SIDE: f32 = 200.0;

    fn wide() -> CropFrame {
        CropFrame::new(Size::new(400.0, 200.0))
    }

    #[test]
    fn a_photo_opens_with_its_middle_and_its_short_side_in_the_mask() {
        let frame = wide();
        assert_eq!(frame.cut(), Rect::new(100.0, 0.0, 200.0, 200.0));
        assert!((frame.scale(SIDE) - 1.0).abs() < f32::EPSILON);
        assert_eq!(frame.picture_rect(SIDE), Rect::new(-200.0, -100.0, 400.0, 200.0));
    }

    #[test]
    fn a_pan_stops_at_the_edge_of_the_photo() {
        let mut frame = wide();
        frame.pan(Point::new(60.0, 0.0), SIDE);
        assert_eq!(frame.cut().origin, Point::new(40.0, 0.0));
        frame.pan(Point::new(500.0, 500.0), SIDE);
        assert_eq!(frame.cut().origin, Point::new(0.0, 0.0));
        frame.pan(Point::new(-900.0, -900.0), SIDE);
        assert_eq!(frame.cut().origin, Point::new(200.0, 0.0));
    }

    #[test]
    fn a_zoom_keeps_the_pixel_under_the_point() {
        let mut frame = wide();
        let at = Point::new(50.0, 50.0);
        let before = frame.focus + at / frame.scale(SIDE);
        frame.zoom_at(at, 2.0, SIDE);
        let after = frame.focus + at / frame.scale(SIDE);
        assert!((before - after).x.abs() < 0.001 && (before - after).y.abs() < 0.001);
        assert!((frame.cut().size.width - 100.0).abs() < 0.001);
    }

    #[test]
    fn the_zoom_stays_between_the_cover_and_the_most() {
        let mut frame = wide();
        frame.zoom_at(Point::default(), 0.1, SIDE);
        assert!((frame.zoom - 1.0).abs() < f32::EPSILON);
        frame.set_zoom(100.0, SIDE);
        assert!((frame.zoom - MAX_ZOOM).abs() < f32::EPSILON);
    }

    #[test]
    fn a_zoom_out_at_the_edge_pulls_the_cut_back_into_the_photo() {
        let mut frame = wide();
        frame.set_zoom(4.0, SIDE);
        frame.pan(Point::new(-900.0, -900.0), SIDE);
        frame.set_zoom(1.0, SIDE);
        let cut = frame.cut();
        assert!(cut.origin.x + cut.size.width <= 400.001);
        assert!(cut.origin.y >= -0.001 && cut.origin.y + cut.size.height <= 200.001);
    }

    #[test]
    fn the_cut_does_not_follow_the_size_of_the_view() {
        let mut frame = wide();
        frame.pan(Point::new(30.0, 0.0), SIDE);
        let cut = frame.cut();
        assert_eq!(cut, frame.cut());
        assert!((frame.scale(SIDE * 2.0) - 2.0 * frame.scale(SIDE)).abs() < 0.001);
    }
}
