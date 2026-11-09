use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, anyhow};
use image::RgbaImage;
use log::{debug, info, warn};
use ui_proc::view;

use super::{
    frame::{CropFrame, MAX_ZOOM},
    pixels,
};
use crate::{
    self as hilen,
    deps::{
        hreads::{on_main, spawn},
        refs::{Weak, manage::DataManager},
        vents::Event,
    },
    gm::{
        LossyConvert,
        color::{BLACK, WHITE},
        flat::{FillRule, Point, Rect, Size, StrokeStyle, VectorPath},
    },
    image::Image,
    ui::{
        DrawingView, Focus, ImageView, NamedKey, Pinch, Scrollable, Setup, Touch, TouchStack, UIEvents,
        UIManager, View, ViewData, ViewFrame, ViewTouch,
    },
};

/// Every photo is kept under a name of its own.
static NEXT_IMAGE: AtomicU64 = AtomicU64::new(0);

/// The dark room around the mask, so the player sees what is left out.
const MARGIN: f32 = 20.0;
/// One wheel notch is 60 pixels, that zooms by about an eighth.
const WHEEL_ZOOM: f32 = 0.002;
/// How far 1 press of an arrow moves the photo, in points.
const KEY_STEP: f32 = 10.0;
/// How much 1 press of `+` or `-` zooms.
const KEY_ZOOM: f32 = 1.15;

/// The shape of the hole in the mask. The cut is a square either way, a
/// round avatar is a square drawn round.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CropShape {
    #[default]
    Circle,
    Square,
}

/// Shows a photo under a mask with a hole, for a player to pick the part
/// that becomes an avatar, like the crop of a profile photo on any big
/// site. The photo always covers the hole.
///
/// A drag moves the photo, the wheel and a pinch zoom it at the pointer.
/// A press on the view, or Enter on the focus ring, gives it the arrow keys
/// to move and `+` and `-` to zoom, the next Enter gives them back.
/// `cut_png` makes the square png of what the hole shows.
#[view]
pub struct ImageCropView {
    shape:      CropShape,
    crop:       CropFrame,
    /// The upright photo, the cut is made from it.
    picture:    Option<RgbaImage>,
    /// The name the photo is kept under as an image, freed with the view.
    image_name: String,
    /// Counts the photos given, a decode that ends after a newer photo came
    /// is dropped.
    loads:      u64,
    drag_from:  Option<Point>,
    keys_held:  bool,
    /// The size the mask was drawn for.
    mask_size:  Size,

    /// The photo given to `set_photo` is in, `false` when its bytes were no
    /// photo the view can read.
    pub loaded:    Event<bool>,
    /// The zoom or the place of the photo changed.
    pub on_change: Event,

    #[init]
    photo: ImageView,
    mask:  DrawingView,
}

impl ImageCropView {
    /// Starts to read the photo in `bytes`, a png, a jpeg, a gif or a webp.
    /// `loaded` tells when it is in. The photo opens with its middle in the
    /// hole and its short side across it.
    pub fn set_photo(mut self: Weak<Self>, bytes: Vec<u8>) {
        self.loads += 1;
        let load = self.loads;
        debug!("Reading a photo of {} bytes to crop", bytes.len());
        spawn(async move {
            let result = pixels::decode(bytes).await;
            on_main(move || self.photo_read(load, result));
        });
    }

    /// Whether a photo is in.
    pub fn has_photo(&self) -> bool {
        self.picture.is_some()
    }

    pub fn shape(&self) -> CropShape {
        self.shape
    }

    pub fn set_shape(&mut self, shape: CropShape) -> &mut Self {
        self.shape = shape;
        self.mask_size = Size::default();
        self.draw_mask();
        self
    }

    /// 1 when the short side of the photo just fills the hole.
    pub fn zoom(&self) -> f32 {
        self.crop.zoom
    }

    /// How far the view zooms in.
    pub fn max_zoom() -> f32 {
        MAX_ZOOM
    }

    /// Zooms and keeps the middle of the hole where it is.
    pub fn set_zoom(&mut self, zoom: f32) -> &mut Self {
        let side = self.hole_side();
        self.crop.set_zoom(zoom, side);
        self.changed();
        self
    }

    /// What the hole shows, a square in pixels of the upright photo. None
    /// while no photo is in.
    pub fn cut(&self) -> Option<Rect> {
        self.picture.as_ref().map(|_| self.crop.cut())
    }

    /// The png of what the hole shows, `side` pixels square.
    pub fn cut_png(&self, side: u32) -> Result<Vec<u8>> {
        let picture = self.picture.as_ref().ok_or_else(|| anyhow!("No photo to cut"))?;
        let cut = self.crop.cut();
        let png = pixels::cut_png(picture, cut, side)?;
        info!(
            "Cut {:.0}x{:.0} at {:.0},{:.0} of the photo into a png of {side} pixels, {} bytes",
            cut.size.width,
            cut.size.height,
            cut.origin.x,
            cut.origin.y,
            png.len()
        );
        Ok(png)
    }

    fn photo_read(mut self: Weak<Self>, load: u64, result: Result<RgbaImage>) {
        if !self.is_ok() {
            return;
        }
        if load != self.loads {
            debug!("A photo to crop came after a newer one, it is dropped");
            return;
        }
        let picture = match result {
            Ok(picture) => picture,
            Err(error) => {
                warn!("Failed to read the photo to crop: {error:#}");
                self.loaded.trigger(false);
                return;
            }
        };
        let (width, height) = picture.dimensions();
        info!("A photo of {width}x{height} is in the crop view");

        let old = std::mem::take(&mut self.image_name);
        self.image_name = format!("image crop {}", NEXT_IMAGE.fetch_add(1, Ordering::Relaxed));
        let image = Image::from_raw_data(
            picture.as_raw().clone(),
            self.image_name.clone(),
            (width, height).into(),
            4,
        );
        self.photo.set_image(image);
        if !old.is_empty() {
            Image::free_with_name(old);
        }

        let size: Size<f32> = Size::new(width.lossy_convert(), height.lossy_convert());
        self.crop = CropFrame::new(size);
        self.picture = Some(picture);
        self.changed();
        self.loaded.trigger(true);
    }

    /// The side of the hole in points.
    fn hole_side(&self) -> f32 {
        let size = self.size();
        (size.width.min(size.height) - MARGIN * 2.0).max(1.0)
    }

    fn middle(&self) -> Point {
        let size = self.size();
        Point::new(size.width / 2.0, size.height / 2.0)
    }

    fn changed(&mut self) {
        self.place_photo();
        self.on_change.trigger(());
    }

    fn place_photo(&mut self) {
        let mut rect = self.crop.picture_rect(self.hole_side());
        rect.origin += self.middle();
        self.photo.set_hidden(self.picture.is_none());
        self.photo.set_frame(rect);
    }

    fn draw_mask(&mut self) {
        let size = self.size();
        if size == self.mask_size && !self.mask.paths().is_empty() {
            return;
        }
        self.mask_size = size;
        self.mask.set_frame((0, 0, size.width, size.height));
        self.mask.remove_all_paths();

        let middle = self.middle();
        let half = self.hole_side() / 2.0;
        let corners = [
            (middle.x - half, middle.y - half),
            (middle.x + half, middle.y - half),
            (middle.x + half, middle.y + half),
            (middle.x - half, middle.y + half),
        ];
        let around = VectorPath::builder()
            .move_to((0, 0))
            .line_to((size.width, 0.0))
            .line_to((size.width, size.height))
            .line_to((0.0, size.height))
            .close();
        let (scrim, edge) = match self.shape {
            CropShape::Circle => (
                around.circle(middle, half).build(),
                VectorPath::circle(middle, half),
            ),
            CropShape::Square => {
                let hole = corners.iter().fold(around, |path, corner| path.line_to(*corner));
                (hole.close().build(), VectorPath::polygon(corners))
            }
        };
        // Over a photo, so the same in both themes.
        self.mask.add_fill(&scrim, BLACK.with_alpha(0.55), FillRule::EvenOdd);
        let width = if self.keys_held { 3.0 } else { 1.5 };
        self.mask.add_stroke(&edge, WHITE.with_alpha(0.9), StrokeStyle::width(width));
    }

    fn drag(&mut self, touch: Touch) {
        if touch.is_ended() {
            self.drag_from = None;
            return;
        }
        if let Some(from) = self.drag_from
            && touch.is_moved()
        {
            let side = self.hole_side();
            self.crop.pan(touch.position - from, side);
            self.changed();
        }
        self.drag_from = Some(touch.position);
    }

    fn pinch(&mut self, pinch: Pinch) {
        let side = self.hole_side();
        self.crop.pan(pinch.shift, side);
        let at = pinch.center - self.middle();
        self.crop.zoom_at(at, pinch.scale, side);
        self.changed();
    }

    fn zoom_by(&mut self, factor: f32) {
        let side = self.hole_side();
        self.crop.zoom_at(Point::default(), factor, side);
        self.changed();
    }

    fn set_keys_held(mut self: Weak<Self>, held: bool) {
        if self.keys_held == held {
            return;
        }
        self.keys_held = held;
        if held {
            Focus::hold_keys(self.weak_view());
        } else {
            Focus::release_keys(self.weak_view());
        }
        self.mask_size = Size::default();
        self.draw_mask();
    }

    fn pointer_pressed(self: Weak<Self>, at: Point) {
        let here = !self.is_hidden_in_tree()
            && self.absolute_frame().contains(at)
            && TouchStack::key_reaches(self.raw());
        self.set_keys_held(here && self.picture.is_some());
    }

    fn keys_active(&self) -> bool {
        self.keys_held
            && !UIManager::text_editing()
            && !self.is_hidden_in_tree()
            && TouchStack::key_reaches(self.weak().raw())
    }

    fn on_key(mut self: Weak<Self>, key: NamedKey) {
        if !self.keys_active() {
            return;
        }
        let side = self.hole_side();
        let step = match key {
            NamedKey::ArrowLeft => Point::new(-KEY_STEP, 0.0),
            NamedKey::ArrowRight => Point::new(KEY_STEP, 0.0),
            NamedKey::ArrowUp => Point::new(0.0, -KEY_STEP),
            NamedKey::ArrowDown => Point::new(0.0, KEY_STEP),
            NamedKey::Enter => {
                self.set_keys_held(false);
                Focus::set(self.weak_view());
                return;
            }
            _ => return,
        };
        self.crop.pan(step, side);
        self.changed();
    }

    fn on_char(mut self: Weak<Self>, character: char) {
        if !self.keys_active() {
            return;
        }
        match character {
            '+' | '=' => self.zoom_by(KEY_ZOOM),
            '-' | '_' => self.zoom_by(1.0 / KEY_ZOOM),
            _ => (),
        }
    }
}

impl Setup for ImageCropView {
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(mut self: Weak<Self>) {
        self.crop = CropFrame::default();
        self.photo.set_hidden(true);

        self.enable_touch();
        self.touch().all.val(move |touch| self.drag(touch));
        self.enable_pinch();
        self.touch().pinch.val(self, move |pinch| self.pinch(pinch));
        TouchStack::enable_scroll(self);

        UIEvents::touch_began().val(self, move |touch| self.pointer_pressed(touch.position));
        UIEvents::keyboard_key().val(self, move |key| self.on_key(key));
        UIEvents::keyboard_input().val(self, move |character| self.on_char(character));

        self.size_changed().sub(move || {
            self.draw_mask();
            self.place_photo();
        });
    }
}

impl Scrollable for ImageCropView {
    fn __process_scroll_touch(&mut self, _touch: Touch) -> bool {
        false
    }

    fn __moves_sideways(&self) -> bool {
        true
    }

    fn __process_wheel_scroll(&mut self, delta: Point) {
        let cursor = self.__base_view().local_point(UIManager::cursor_position());
        let at = cursor - self.middle();
        let side = self.hole_side();
        self.crop.zoom_at(at, (delta.y * WHEEL_ZOOM).exp(), side);
        self.changed();
    }
}

/// No weak pointer of the view here, it is no longer counted while it is
/// freed. The keys need nothing, `Focus` drops a holder that is gone.
impl Drop for ImageCropView {
    fn drop(&mut self) {
        if !self.image_name.is_empty() {
            Image::free_with_name(std::mem::take(&mut self.image_name));
        }
    }
}

#[cfg(feature = "ui-tests")]
#[path = "crop_view_test.rs"]
mod test;
